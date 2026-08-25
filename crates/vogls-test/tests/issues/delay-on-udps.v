// vogls: fail=lower
// vogls: mode=two-value-logic
// vogls: backend=bytecode
primitive my_and(z, a, b);
    output z;
    input a, b;
table
    1 1 : 1 ;
    0 ? : 0 ;
    ? 0 : 0 ;
endtable
endprimitive

module tb();
    reg a, b;
    wire z;

    my_and #3 g1 (z, a, b);
endmodule
