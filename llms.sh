#!/usr/bin/env bash

cd "$(dirname "$0")"

llms . "*.txt,01code,02book,03fonts,AGENTS.md,CLAUDE.md,GEMINI.md,LLXPRT.md,QWEN.md,WORK.md,issues,test_results.txt,external,*.html,01code-tldr.txt,LICENSE"
