module top_module (
    input [4:0] a, b, c, d, e, f,
    output [7:0] w, x, y, z
);
    assign { w, x, y, z } = { a, b, c, d, e, f, 2'b11 };
endmodule

module tb();
    reg [4:0] a, b, c, d, e, f;
    wire [7:0] w, x, y, z;

    top_module m(a, b, c, d, e, f, w, x, y, z);

    initial begin
        #1 a = 5'h0; b = 5'h0; c = 5'h0; d = 5'h0; e = 5'h0; f = 5'h0;
        #1
        $vogls_assert_eq(w, 8'h00);
        $vogls_assert_eq(x, 8'h00);
        $vogls_assert_eq(y, 8'h00);
        $vogls_assert_eq(z, 8'h03);

        #1 a = 5'h1F; b = 5'h1F; c = 5'h1F; d = 5'h1F; e = 5'h1F; f = 5'h1F;
        #1
        $vogls_assert_eq(w, 8'hFF);
        $vogls_assert_eq(x, 8'hFF);
        $vogls_assert_eq(y, 8'hFF);
        $vogls_assert_eq(z, 8'hFF);

        #1 a = 5'hA; b = 5'hB; c = 5'hC; d = 5'hD; e = 5'hE; f = 5'hF;
        #1
        $vogls_assert_eq(w, 8'h52);
        $vogls_assert_eq(x, 8'hD8);
        $vogls_assert_eq(y, 8'hD7);
        $vogls_assert_eq(z, 8'h3F);
    end
endmodule
