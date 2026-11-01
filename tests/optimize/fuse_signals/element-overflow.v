// vogls: fail=execute
// @TODO: A bit- or part-select past the end of an array element reads the neighbouring element
// instead of x. Fusing rejects these, but the normal expression lowering has the same bug, even
// in procedural code (array index overflow is checked, the select within the element is not).
module top;
    wire [7:0] mem [0:3];
    assign mem[0] = 8'h11;
    assign mem[1] = 8'h22;
    assign mem[2] = 8'h33;
    assign mem[3] = 8'h44;

    wire [7:0] lo [1:2];
    assign lo[1] = 8'hAB;
    assign lo[2] = 8'hCD;

    wire past_bit = mem[0][9];
    wire [3:0] past_part = mem[1][9:6];
    wire below_bit = mem[1][-1];
    wire [3:0] past_lo = lo[1][9:6];

    initial begin
        #1;
`ifdef __VOGLS__TWO_VALUE_LOGIC
        $vogls_assert_eq(past_bit, 1'b0);
        $vogls_assert_eq(past_part, 4'h0);
        $vogls_assert_eq(below_bit, 1'b0);
        $vogls_assert_eq(past_lo, 4'h2);
        $vogls_assert_eq(mem[0][9], 1'b0);
`else
        $vogls_assert_eq(past_bit, 1'bx);
        $vogls_assert_eq(past_part, 4'bxx00);
        $vogls_assert_eq(below_bit, 1'bx);
        $vogls_assert_eq(past_lo, 4'bxx10);
        $vogls_assert_eq(mem[0][9], 1'bx);
`endif
    end
endmodule
