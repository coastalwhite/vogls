module x();
	wire [ 71:0 ] LUT;
	assign LUT = { 8'h63, 8'h7c, 56'b0 };
    integer i;
    initial begin
        for (i = 0; i <= 1; i = i + 1) begin
            if (i == 0)
                $vogls_assert_eq(8'h63, LUT[71-8*i-:8]);
            else 
                $vogls_assert_eq(8'h7c, LUT[71-8*i-:8]);
        end
    end
endmodule
