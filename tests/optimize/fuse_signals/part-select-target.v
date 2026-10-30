// A wider value assigned to a part-select is truncated to the part-select, not to the rest of
// the net.
module top;
    reg [7:0] b;
    reg [3:0] c;
    wire [15:0] a;

    assign a[3:0] = b;
    assign a[7:4] = c;
    assign a[15:8] = 8'h00;

    initial begin
        b = 8'hA5;
        c = 4'h3;
        #1;
        $vogls_assert_eq(a, 16'h0035);
        c = 4'hC;
        #1;
        $vogls_assert_eq(a, 16'h00C5);
    end
endmodule
