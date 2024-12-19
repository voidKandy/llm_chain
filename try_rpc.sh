if [ -z "$1" ]; then
    echo "Usage: $0 <command>"
    exit 1
fi

COMMAND=$1


SESSION_NAME=$(tmux display-message -p '#S')
WINDOW_ID=$(tmux display-message -p '#I')
PANE_COUNT=$(tmux list-panes -t $SESSION_NAME:$WINDOW_ID | wc -l)

if [ "$PANE_COUNT" -eq 1 ]; then
    tmux split-window -h
    tmux select-pane -t $SESSION_NAME:$WINDOW_ID.1
    tmux split-window -v
fi

if [ "$PANE_COUNT" -eq 2 ]; then
    tmux select-pane -t $SESSION_NAME:$WINDOW_ID.1
    tmux split-window -v
fi

ADDR="127.0.0.1:3000";

tmux send-keys -t $SESSION_NAME:$WINDOW_ID.1 C-c
tmux send-keys -t $SESSION_NAME:$WINDOW_ID.1 "clear" C-m

tmux send-keys -t $SESSION_NAME:$WINDOW_ID.2 C-c
tmux send-keys -t $SESSION_NAME:$WINDOW_ID.2 "clear" C-m

tmux send-keys -t $SESSION_NAME:$WINDOW_ID.1 "cargo run --bin node -- -a $ADDR boot | bunyan" C-m

sleep 1.8

tmux send-keys -t $SESSION_NAME:$WINDOW_ID.2 "cargo run --bin rpc -- -a $ADDR $COMMAND | bunyan" C-m