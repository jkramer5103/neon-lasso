#!/usr/bin/env bash
# Drives the game inside tmux with scripted input and captures frames as PNGs.
set -u
cd "$(dirname "$0")/.."
COLS=${COLS:-160}; ROWS=${ROWS:-48}
OUT=${OUT:-shots}
mkdir -p "$OUT"
tmux kill-session -t g 2>/dev/null
tmux -f /dev/null new-session -d -s g -x "$COLS" -y "$ROWS"
tmux set -g default-terminal tmux-256color
tmux set -ga terminal-overrides ',*:RGB'
tmux send-keys -t g "COLORTERM=truecolor ./target/release/neon-lasso" Enter
sleep 3.5
k() { tmux send-keys -t g "$1"; sleep "$2"; }
cap() { tmux capture-pane -t g -e -p > "$OUT/$1.ans"; python3 tools/ansi2png.py "$OUT/$1.ans" "$OUT/$1.png" "$COLS" "$ROWS"; }
k Enter 0.3
cap start
# lasso the opening swarm: right, up, left, down, right
sleep 0.05
k Up 0.75
k Left 0.68
cap loop_mid
k Down 0.72
k Right 0.30
cap loop_a
sleep 0.08; cap loop_b
sleep 0.15; cap loop_c
sleep 0.35; cap loop_d
sleep 0.6; cap after
for i in 1 2 3 4 5 6; do
  k Up 0.45; k Left 0.45; k Down 0.45; k Right 0.45
done
cap later
k p 0.3
cap paused
k p 0.2
sleep 8
cap late
tmux send-keys -t g q
sleep 0.5
tmux kill-session -t g 2>/dev/null
