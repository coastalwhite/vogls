module aes_scale_omega2_gf2p2(data_i, data_o);
    input  [1:0] data_i;
    output [1:0] data_o;

    assign data_o = { data_i[0], ^data_i };
endmodule

module tb();
    reg [1:0] i;
    wire [1:0] o;

    aes_scale_omega2_gf2p2 s(i, o);

    initial begin
        i = 2'b00;
        #1
        $vogls_assert_eq(o, 2'b00);
        #1

        i = 2'b01;
        #1
        $vogls_assert_eq(o, 2'b11);
        #1

        i = 2'b10;
        #1
        $vogls_assert_eq(o, 2'b01);
        #1

        i = 2'b11;
        #1
        $vogls_assert_eq(o, 2'b10);
    end
endmodule
