module top();
	localparam one_baud_cnt = 20833;
    reg [log2(one_baud_cnt * 16)-1:0] rx_clk;

    function integer log2(input integer M);
        integer i;
    begin
        log2 = 1;
        for (i = 0; 2**i <= M; i = i + 1)
            log2 = i + 1;
    end endfunction

	initial begin
        $vogls_assert_eq(log2(20833 * 16), 19);
		$vogls_assert_eq($bits(rx_clk), 19);
	end
endmodule
