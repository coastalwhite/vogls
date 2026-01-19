module A ( input x, input y, output z ); 
    assign z = (x^y) & x;
endmodule
module B ( input x, input y, output z ); 
    assign z = ~(x^y);
endmodule
module top_module ( input x, input y, output z ); 
    wire a0w, a1w, b0w, b1w;

    A a0 ( x, y, a0w );
    A a1 ( x, y, a1w );
    B b0 ( x, y, b0w );
    B b1 ( x, y, b1w );

    assign z = (a0w | b0w) ^ (a1w & b1w);
endmodule

module tb();
    reg x, y;
    wire z;

    top_module i( .z(z), .y(y), .x(x) );

    initial begin
        x=0;y=0; #1 $vogls_assert_eq(z, 1);
        x=0;y=1; #1 $vogls_assert_eq(z, 0);
        x=1;y=0; #1 $vogls_assert_eq(z, 1);
        x=1;y=1; #1 $vogls_assert_eq(z, 1);
    end
endmodule
