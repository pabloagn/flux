#!/usr/bin/env bash

# Monitor QuestDB performance
watch -n 1 '
echo "=== QuestDB Status ==="
echo ""
echo "Table Sizes:"
curl -s -G http://localhost:9000/exec \
    --data-urlencode "query=SELECT table_name, row_count FROM tables()" | \
    jq -r ".dataset[]" 2>/dev/null

echo ""
echo "Recent Data (last 10 seconds):"
curl -s -G http://localhost:9000/exec \
    --data-urlencode "query=SELECT count(*) as count FROM cell_metrics WHERE ts > dateadd('\''s'\'', -10, now())" | \
    jq -r ".dataset[0][0]" 2>/dev/null

echo ""
echo "Writer Metrics:"
curl -s http://localhost:8000/metrics | grep questdb_ | head -10
'
