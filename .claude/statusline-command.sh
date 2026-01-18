#!/bin/bash
# RuVector Intelligence Statusline - Multi-line display
INPUT=$(cat)
MODEL=$(echo "$INPUT" | jq -r '.model.display_name // "Claude"')
CWD=$(echo "$INPUT" | jq -r '.workspace.current_dir // .cwd')
DIR=$(basename "$CWD")
BRANCH=$(cd "$CWD" 2>/dev/null && git branch --show-current 2>/dev/null)
RESET="\033[0m"; BOLD="\033[1m"; CYAN="\033[36m"; YELLOW="\033[33m"; GREEN="\033[32m"; MAGENTA="\033[35m"; BLUE="\033[34m"; DIM="\033[2m"; RED="\033[31m"
printf "$BOLD$MODEL$RESET in $CYAN$DIR$RESET"
[ -n "$BRANCH" ] && printf " on $YELLOW⎇ $BRANCH$RESET"
echo
INTEL_FILE=""
for P in "$CWD/.ruvector/intelligence.json" "$CWD/npm/packages/ruvector/.ruvector/intelligence.json" "$HOME/.ruvector/intelligence.json"; do
  [ -f "$P" ] && INTEL_FILE="$P" && break
done
if [ -n "$INTEL_FILE" ]; then
  INTEL=$(cat "$INTEL_FILE" 2>/dev/null)
  MEMORY_COUNT=$(echo "$INTEL" | jq -r '.memories | length // 0' 2>/dev/null)
  TRAJ_COUNT=$(echo "$INTEL" | jq -r '.trajectories | length // 0' 2>/dev/null)
  SESSION_COUNT=$(echo "$INTEL" | jq -r '.stats.session_count // 0' 2>/dev/null)
  PATTERN_COUNT=$(echo "$INTEL" | jq -r '.patterns | length // 0' 2>/dev/null)
  printf "$MAGENTA🧠 RuVector$RESET"
  [ "$PATTERN_COUNT" != "null" ] && [ "$PATTERN_COUNT" -gt 0 ] 2>/dev/null && printf " $GREEN◆$RESET $PATTERN_COUNT patterns" || printf " $DIM◇ learning$RESET"
  [ "$MEMORY_COUNT" != "null" ] && [ "$MEMORY_COUNT" -gt 0 ] 2>/dev/null && printf " $BLUE⬡$RESET $MEMORY_COUNT mem"
  [ "$TRAJ_COUNT" != "null" ] && [ "$TRAJ_COUNT" -gt 0 ] 2>/dev/null && printf " $YELLOW↝$RESET$TRAJ_COUNT"
  [ "$SESSION_COUNT" != "null" ] && [ "$SESSION_COUNT" -gt 0 ] 2>/dev/null && printf " $DIM#$SESSION_COUNT$RESET"
  echo
else
  printf "$DIM🧠 RuVector: run 'npx ruvector hooks session-start' to initialize$RESET\n"
fi
