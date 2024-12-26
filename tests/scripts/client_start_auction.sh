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
tmux send-keys -t $SESSION_NAME:$WINDOW_ID.1 "cargo run --bin server -- -b provider | bunyan" C-m
sleep 1.8

tmux send-keys -t $SESSION_NAME:$WINDOW_ID.2 "cd client" C-m
tmux send-keys -t $SESSION_NAME:$WINDOW_ID.2 "cargo run --bin client -- -a $ADDR | bunyan" C-m
sleep 1.8

tmux send-keys -t $SESSION_NAME:$WINDOW_ID.0 "cd client" C-m
tmux send-keys -t $SESSION_NAME:$WINDOW_ID.0 "cargo run --bin rpc -- -a $ADDR start-auction | bunyan" C-m

# tmux send-keys -t $SESSION_NAME:$WINDOW_ID.2 "cargo run --bin rpc -- -a $ADDR $COMMAND | bunyan" C-m
