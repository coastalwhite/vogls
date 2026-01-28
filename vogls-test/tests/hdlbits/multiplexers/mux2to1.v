module top_module( 
    input a, b, sel,
    output out ); 
    assign out = sel ? b : a;
endmodule

module tb();
    reg a, b, sel;
    wire out;

    top_module i(a, b, sel, out);

    initial begin
        a=0;b=0;sel=0; #1 $vogls_assert_eq(out, 0);
        // a=0;b=0;sel=1; #1 $vogls_assert_eq(out, 0);
        // a=0;b=1;sel=0; #1 $vogls_assert_eq(out, 0);
        // a=0;b=1;sel=1; #1 $vogls_assert_eq(out, 1);
        // a=1;b=0;sel=0; #1 $vogls_assert_eq(out, 1);
        // a=1;b=0;sel=1; #1 $vogls_assert_eq(out, 0);
        // a=1;b=1;sel=0; #1 $vogls_assert_eq(out, 1);
        // a=1;b=1;sel=1; #1 $vogls_assert_eq(out, 1);
    end
endmodule
