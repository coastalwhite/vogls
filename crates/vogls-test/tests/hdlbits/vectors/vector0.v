module top_module(
    input wire [2:0] vec,
    output wire [2:0] outv,
    output wire o2,
    output wire o1,
    output wire o0 );

    assign outv = vec;
    assign o2 = vec[2];
    assign o1 = vec[1];
    assign o0 = vec[0];
endmodule

module tb();
    reg [2:0] vec;
    wire [2:0] outv;
    wire o2;
    wire o1;
    wire o0;

    top_module m(vec, outv, o2, o1, o0);

    integer i = 0;
    initial begin
        #1 vec = 3'h3;
        #1
        $vogls_assert_eq(outv, 3'h3);
        $vogls_assert_eq(o2, 1'b0);
        $vogls_assert_eq(o1, 1'b1);
        $vogls_assert_eq(o0, 1'b1);

        #1 vec = 3'h5;
        #1
        $vogls_assert_eq(outv, 3'h5);
        $vogls_assert_eq(o2, 1'b1);
        $vogls_assert_eq(o1, 1'b0);
        $vogls_assert_eq(o0, 1'b1);

        for (i = 0; i < 8; i = i + 1) begin
            #1 vec = i;
            #1
            $vogls_assert_eq(outv, i);
            $vogls_assert_eq(o2, i[2]);
            $vogls_assert_eq(o1, i[1]);
            $vogls_assert_eq(o0, i[0]);
        end
    end
endmodule
