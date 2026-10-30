// Selects past the end of a vector or array must not be fused onto bits that do not exist.
module top;
    reg [7:0] r;
    wire [7:0] mem [0:3];
    assign mem[0] = 8'h11;
    assign mem[1] = 8'h22;
    assign mem[2] = 8'h33;
    assign mem[3] = 8'h44;

    // Valid selects into array elements are still fused.
    wire [3:0] in_elem = mem[1][7:4];
    wire in_bit = mem[2][0];

    // An element past the end of the array.
    wire [7:0] past_array = mem[5];

    // Writes past the end of the vector are dropped.
    wire [7:0] w;
    assign w[9:6] = 4'hF;
    assign w[5:0] = 6'h0;

    // Reads past the end of the vector are x.
    wire [3:0] v = r[9:6];

    initial begin
        r = 8'hA5;
        #1;
        $vogls_assert_eq(in_elem, 4'h2);
        $vogls_assert_eq(in_bit, 1'b1);
        $vogls_assert_eq(w, 8'hC0);
`ifdef __VOGLS__TWO_VALUE_LOGIC
        $vogls_assert_eq(past_array, 8'h00);
        $vogls_assert_eq(v, 4'h2);
`else
        $vogls_assert_eq(past_array, 8'hxx);
        $vogls_assert_eq(v, 4'bxx10);
`endif
    end
endmodule
