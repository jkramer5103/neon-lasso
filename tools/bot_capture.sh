#!/usr/bin/env bash
# Runs the game with the lasso bot playing, captures frames at an interval.
set -u
cd "$(dirname "$0")/.."
COLS=${COLS:-160}; ROWS=${ROWS:-48}; OUT=${OUT:-shots/bot}; N=${N:-80}; DT=${DT:-0.33}
rm -rf "$OUT"; mkdir -p "$OUT"
tmux kill-session -t b 2>/dev/null
tmux -f /dev/null new-session -d -s b -x "$COLS" -y "$ROWS"
tmux set -g default-terminal tmux-256color
tmux set -ga terminal-overrides ',*:RGB'
tmux send-keys -t b "COLORTERM=truecolor NEON_LASSO_BOT=${BOT:-1} ./target/release/neon-lasso" Enter
sleep 3.5
tmux send-keys -t b Enter
for i in $(seq -w 1 "$N"); do
  sleep "$DT"
  tmux capture-pane -t b -e -p > "$OUT/f$i.ans"
done
ps -o %cpu= -C neon-lasso | head -1 | sed 's/^/cpu% /'
tmux send-keys -t b q; sleep 0.3; tmux kill-session -t b 2>/dev/null
for f in "$OUT"/*.ans; do python3 tools/ansi2png.py "$f" "${f%.ans}.png" "$COLS" "$ROWS"; done
for f in "$OUT"/*.ans; do printf "%s %s\n" "$(basename "$f" .ans)" "$(head -1 "$f" | sed 's/\x1b\[[0-9;]*m//g' | tr -s ' ' | cut -c1-90)"; done
