// vogls: backend=bytecode
// vogls: mode=four-value-logic
// vogls: fail=parse
module tb();
    wire a, b;
    tran #1 t (a, b);
endmodule
