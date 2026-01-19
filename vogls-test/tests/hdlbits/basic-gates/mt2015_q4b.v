module top_module ( input x, input y, output z ); 
    assign z = ~(x^y);
endmodule

module tb();
    reg x, y;
    wire z;

    top_module i( .z(z), .y(y), .x(x) );

    initial begin
        x=0;y=0; #1 $vogls_assert_eq(z, 1);
        x=0;y=1; #1 $vogls_assert_eq(z, 0);
        x=1;y=0; #1 $vogls_assert_eq(z, 0);
        x=1;y=1; #1 $vogls_assert_eq(z, 1);
    end
endmodule
