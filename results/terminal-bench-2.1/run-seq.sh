#!/bin/bash
# Terminal-Bench 2.1, SEQUENTIAL (representative): 1 server slot at full context, 1 task at a time.
# opencode first, then Codex. Resume after a pause with resume-seq.sh.
cd ~/tb; K=$(cat tb-key)
(setsid nohup ~/tb/start-tb-server-seq.sh > ~/tb/server.log 2>&1 < /dev/null &)
for i in $(seq 1 120); do [ "$(timeout 3 curl -s -o /dev/null -w '%{http_code}' http://172.17.0.1:8090/health)" = 200 ] && break; sleep 2; done
OC=$(python3 -c "
import json; print(json.dumps({'provider':{'local':{'npm':'@ai-sdk/openai-compatible','name':'local','options':{'baseURL':'http://172.17.0.1:8090/v1','apiKey':'$K'},'models':{'local':{'name':'Qwen3.8-27B','limit':{'context':262144,'output':32768}}}}},'permission':{'external_directory':'allow'}}))")
if [ ! -f ~/tb/jobs/tb21seq-opencode-27b/config.json ]; then
  echo "[$(date +%H:%M)] opencode start"
  harbor run -p ~/tb/terminal-bench-2-1 -a opencode -m local/local --ak "opencode_config=$OC" -n 1 -o ~/tb/jobs --job-name tb21seq-opencode-27b > ~/tb/seq-opencode.log 2>&1
else
  echo "[$(date +%H:%M)] opencode resume"
  harbor jobs resume -p ~/tb/jobs/tb21seq-opencode-27b -f AgentSetupTimeoutError -f NonZeroAgentExitCodeError > ~/tb/seq-opencode.log 2>&1
fi
echo "[$(date +%H:%M)] opencode done (exit $?)"
if [ ! -f ~/tb/jobs/tb21seq-codex-27b/config.json ]; then
  harbor run -p ~/tb/terminal-bench-2-1 -a codex -m local --ak "config=$HOME/tb/codex-config-seq.toml" --ae "TB_LOCAL_KEY=$K" -n 1 -o ~/tb/jobs --job-name tb21seq-codex-27b > ~/tb/seq-codex.log 2>&1
else
  harbor jobs resume -p ~/tb/jobs/tb21seq-codex-27b -f AgentSetupTimeoutError -f NonZeroAgentExitCodeError > ~/tb/seq-codex.log 2>&1
fi
echo "[$(date +%H:%M)] codex done (exit $?)"
echo TB-SEQ-DONE
