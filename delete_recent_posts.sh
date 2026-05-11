#!/bin/bash
set -e

# Load .env if it exists
if [ -f .env ]; then
  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ ! "$line" =~ ^# ]] && [[ "$line" =~ = ]]; then
      export "$line"
    fi
  done < .env
fi

if [ -z "$NEWSPENGUIN_MSTD_ACCESS_TOKEN" ]; then
  echo "Error: NEWSPENGUIN_MSTD_ACCESS_TOKEN not set."
  exit 1
fi

API_BASE="${NEWSPENGUIN_MSTD_API_URI:-https://mstd.seungjin.net}"
API_BASE="${API_BASE%/}" # Remove trailing slash

# 1. Get account ID
ACCOUNT_ID=$(curl -s -H "Authorization: Bearer $NEWSPENGUIN_MSTD_ACCESS_TOKEN" "$API_BASE/api/v1/accounts/verify_credentials" | jq -r '.id')

if [ -z "$ACCOUNT_ID" ] || [ "$ACCOUNT_ID" == "null" ]; then
  echo "Error: Could not get account ID."
  exit 1
fi

echo "Account ID: $ACCOUNT_ID"

TOTAL_DELETED=0

while true; do
  FIVE_HOURS_AGO=$(date -u -d "24 hours ago" +%s)
  echo "Searching for posts newer than $(date -u -d "3 hours ago") (Timestamp: $FIVE_HOURS_AGO)"

  # 2. Get statuses
  STATUSES=$(curl -s -H "Authorization: Bearer $NEWSPENGUIN_MSTD_ACCESS_TOKEN" "$API_BASE/api/v1/accounts/$ACCOUNT_ID/statuses?limit=40")
  
  # Filter statuses that match the criteria
  MATCHING_STATUSES=$(echo "$STATUSES" | jq -c ".[] | select((.created_at | sub(\"\\\\.[0-9]+Z$\"; \"Z\") | fromdateiso8601) > $FIVE_HOURS_AGO)")
  
  if [ -z "$MATCHING_STATUSES" ]; then
    echo "No more matching posts found."
    break
  fi

  COUNT_IN_BATCH=0
  while read -r status; do
    if [ -z "$status" ]; then continue; fi
    ID=$(echo "$status" | jq -r '.id')
    CREATED_AT=$(echo "$status" | jq -r '.created_at')
    CONTENT=$(echo "$status" | jq -r '.content' | sed 's/<[^>]*>//g' | head -c 100)
    
    echo "Deleting status $ID (Created at: $CREATED_AT): $CONTENT..."
    HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE -H "Authorization: Bearer $NEWSPENGUIN_MSTD_ACCESS_TOKEN" "$API_BASE/api/v1/statuses/$ID")
    echo "  Result: HTTP $HTTP_STATUS"
    
    if [ "$HTTP_STATUS" == "429" ]; then
      echo "Rate limited. Sleeping for 30 seconds..."
      sleep 30
      # Retry this one? For now just continue and hope next batch works
    elif [ "$HTTP_STATUS" == "200" ] || [ "$HTTP_STATUS" == "204" ]; then
      TOTAL_DELETED=$((TOTAL_DELETED + 1))
      COUNT_IN_BATCH=$((COUNT_IN_BATCH + 1))
    fi
    sleep 1
  done <<EOF
$MATCHING_STATUSES
EOF

  echo "Deleted $COUNT_IN_BATCH posts in this batch. Total deleted so far: $TOTAL_DELETED"
  
  if [ "$COUNT_IN_BATCH" -eq 0 ]; then
    echo "Could not delete any more posts in this batch (maybe rate limited or errors). Stopping to avoid infinite loop."
    break
  fi
  
  # Brief pause before next fetch
  sleep 2
done

echo "Operation complete. Total deleted: $TOTAL_DELETED"
