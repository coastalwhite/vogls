module top();
    reg a, b, c;
    reg [1:0] e;
    wire [1:0] ew;

    assign ew[0] = a;
    assign ew[1] = b;
    assign ew = e;

    initial begin
        #1 e = 2'b11;
`ifdef __VOGLS__TWO_VALUE_LOGIC
        $vogls_assert_eq(a, 1'b0);
        $vogls_assert_eq(b, 1'b0);
`else
        $vogls_assert_eq(a, 1'bx);
        $vogls_assert_eq(b, 1'bx);
`endif
    end
endmodule
