module top_module ( input [1:0] A, input [1:0] B, output z ); 
    assign z = A == B ? 1'b1 : 1'b0;
endmodule

module tb();
    reg [1:0] A, B;
    wire z;

    top_module i( .B(B), .z(z), .A(A) );

    initial begin
        A=2'b00;B=2'b00; #1 $vogls_assert_eq(z, 1'b1);
        A=2'b00;B=2'b01; #1 $vogls_assert_eq(z, 1'b0);
        A=2'b00;B=2'b10; #1 $vogls_assert_eq(z, 1'b0);
        A=2'b00;B=2'b11; #1 $vogls_assert_eq(z, 1'b0);
        A=2'b01;B=2'b00; #1 $vogls_assert_eq(z, 1'b0);
        A=2'b01;B=2'b01; #1 $vogls_assert_eq(z, 1'b1);
        A=2'b01;B=2'b10; #1 $vogls_assert_eq(z, 1'b0);
        A=2'b01;B=2'b11; #1 $vogls_assert_eq(z, 1'b0);
        A=2'b10;B=2'b00; #1 $vogls_assert_eq(z, 1'b0);
        A=2'b10;B=2'b01; #1 $vogls_assert_eq(z, 1'b0);
        A=2'b10;B=2'b10; #1 $vogls_assert_eq(z, 1'b1);
        A=2'b10;B=2'b11; #1 $vogls_assert_eq(z, 1'b0);
        A=2'b11;B=2'b00; #1 $vogls_assert_eq(z, 1'b0);
        A=2'b11;B=2'b01; #1 $vogls_assert_eq(z, 1'b0);
        A=2'b11;B=2'b10; #1 $vogls_assert_eq(z, 1'b0);
        A=2'b11;B=2'b11; #1 $vogls_assert_eq(z, 1'b1);
    end
endmodule
