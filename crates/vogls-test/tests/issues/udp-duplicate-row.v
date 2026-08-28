// This should fail on the duplicate row.
primitive duprow(z, a, b);
    output z;
    input a, b;
table
    0 0 : 0 ;
    0 0 : 1 ;
    ? ? : x ;
endtable
endprimitive

module tb();
    reg a, b;
    wire z;

    duprow (z, a, b);
endmodule
