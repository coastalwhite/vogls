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
    reg [1:0] a, b;
    wire [1:0] z;

    my_and g [1:0] (z, a, b);

    initial begin
        #0 a = 2'b10; b = 2'b11;
        #5 $vogls_assert_eq(z, 2'b10);
    end
endmodule
