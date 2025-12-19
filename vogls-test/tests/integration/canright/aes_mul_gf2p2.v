`ifndef NO_TB
module tb();
    reg [1:0] a_i;
    reg [1:0] b_i;
    wire [1:0] z_i;

    aes_mul_gf2p2 m(a_i, b_i, z_i);

    initial begin
        a_i = 2'b00;
        b_i = 2'b00;
        #1
        $vogls_assert_eq(z_i, 2'b00);
        #1

        a_i = 2'b01;
        b_i = 2'b00;
        #1
        $vogls_assert_eq(z_i, 2'b00);
        #1

        a_i = 2'b10;
        b_i = 2'b00;
        #1
        $vogls_assert_eq(z_i, 2'b00);
        #1

        a_i = 2'b11;
        b_i = 2'b00;
        #1
        $vogls_assert_eq(z_i, 2'b00);
        #1

        a_i = 2'b00;
        b_i = 2'b01;
        #1
        $vogls_assert_eq(z_i, 2'b00);
        #1

        a_i = 2'b01;
        b_i = 2'b01;
        #1
        $vogls_assert_eq(z_i, 2'b10);
        #1

        a_i = 2'b10;
        b_i = 2'b01;
        #1
        $vogls_assert_eq(z_i, 2'b11);
        #1

        a_i = 2'b11;
        b_i = 2'b01;
        #1
        $vogls_assert_eq(z_i, 2'b01);
        #1

        a_i = 2'b00;
        b_i = 2'b10;
        #1
        $vogls_assert_eq(z_i, 2'b00);
        #1

        a_i = 2'b01;
        b_i = 2'b10;
        #1
        $vogls_assert_eq(z_i, 2'b11);
        #1

        a_i = 2'b10;
        b_i = 2'b10;
        #1
        $vogls_assert_eq(z_i, 2'b01);
        #1

        a_i = 2'b11;
        b_i = 2'b10;
        #1
        $vogls_assert_eq(z_i, 2'b10);
        #1

        a_i = 2'b00;
        b_i = 2'b11;
        #1
        $vogls_assert_eq(z_i, 2'b00);
        #1

        a_i = 2'b01;
        b_i = 2'b11;
        #1
        $vogls_assert_eq(z_i, 2'b01);
        #1

        a_i = 2'b10;
        b_i = 2'b11;
        #1
        $vogls_assert_eq(z_i, 2'b10);
        #1

        a_i = 2'b11;
        b_i = 2'b11;
        #1
        $vogls_assert_eq(z_i, 2'b11);
        #1
    end

endmodule
`endif

module aes_mul_gf2p2(a_i, b_i, z_o);
    input [1:0]  a_i;
    input [1:0]  b_i;
    output [1:0] z_o;

    wire a, b, c;

    assign a = a_i[1] & b_i[1];
    assign b = ^a_i & ^b_i;
    assign c = a_i[0] & b_i[0];

    assign z_o = { a ^ b, c ^ b };
endmodule
