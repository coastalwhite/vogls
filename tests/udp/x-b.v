// vogls: mode=four-value-logic

primitive lvl(z, a);
    output z;
    input a;
table
    x : 1 ;
    b : 0 ;
endtable
endprimitive

module tb();
    reg a;
    wire z;

    lvl (z, a);

    initial begin
        #0
        a = 1'b0; #5 $vogls_assert_eq(z, 0);
        a = 1'b1; #5 $vogls_assert_eq(z, 0);
        a = 1'bx; #5 $vogls_assert_eq(z, 1);
        a = 1'bz; #5 $vogls_assert_eq(z, 1);
    end
endmodule
