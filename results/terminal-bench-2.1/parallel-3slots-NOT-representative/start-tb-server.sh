#!/bin/bash
# Terminal-Bench server: 27b like the production profile, but 3 slots x 64k for parallel tasks,
# bound to the Docker bridge only, with a throwaway key (logs will be public).
. ~/llama-env.sh
exec ~/llama.cpp/build/bin/llama-server -m /home/gud/models/Qwen3.8-27B-UD-Q4_K_XL.gguf \
  -md /home/gud/models/drafts/Qwen3.8-27B-DFlash2-Q4_K_M.gguf -ngld 99 -ctkd q8_0 -ctvd q8_0 \
  --spec-type ngram-mod,draft-dflash --spec-draft-n-max 7 \
  -c 196608 -np 3 -ngl 99 -fa on --cache-type-k q8_0 --cache-type-v q8_0 \
  --jinja --chat-template-file /home/gud/models/qwen3.8-chat-template.jinja --reasoning-effort medium \
  --temp 1.0 --top-p 0.95 --top-k 20 --min-p 0 \
  --host 172.17.0.1 --port 8090 --api-key $(cat ~/tb/tb-key) --alias local
