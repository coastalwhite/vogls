#!/bin/sh

echo -n '' > log-verilator
echo -n '' > log-icarus
echo -n '' > log-vogls-interpret
echo -n '' > log-vogls-compile

for it in `seq 1 2`; do
    echo "Iteration $it"
    just clean
    just build

    echo "Running Verilator..."
    just run-verilator       2>&1 | tee -a log-verilator
    echo "Running Icarus Verilog..."
    just run-icarus          2>&1 | tee -a log-icarus
	echo "Running Vogls (Interpret)..."
    just run-vogls-interpret 2>&1 | tee -a log-vogls-interpret
	echo "Running Vogls (Compile)..."
    just run-vogls-compile   2>&1 | tee -a log-vogls-compile
done