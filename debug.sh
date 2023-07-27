#!/bin/sh

GAME="linux_64_client"
LIBPATH="$(pwd)/target/debug/librustycube.so"
LOAD_MODE=1 # RTLD_LAZY

if [ $(id -u) -ne 0 ]; then
	echo "[RC] Please run as root"
	exit 1
fi

echo "[RC] Target process: $GAME"
echo "[RC] Library path: $LIBPATH"

if [ ! -f "$LIBPATH" ]; then
	echo "[RC] Library does not exist"
	exit 1
fi

PID="$(pidof -s $GAME)"
if [ -z "$PID" ]; then
	echo "[RC] The game is not running"
	exit 1
fi

echo "[RC] Target process ID: $PID"

echo "[RC] Injecting..."
gdb -n -q \
	-ex "attach $PID" \
	-ex "set \$dlopen = (void *(*)(char *, int))dlopen" \
	-ex "call \$dlopen(\"$LIBPATH\", $LOAD_MODE)"