// vogls: verify-stdout
module test;
    initial begin
        #0
        $display("# 1");
		$display("%0d", $random);
		$display("%0d", $random);
		$display("%0d", $random);
		$display("%0d", $random);
		$display("%0d", $random);
	end

    initial begin
        #1
        $display("# 2");
		$display("%0d", $random);
		$display("%0d", $random);
		$display("%0d", $random);
		$display("%0d", $random);
		$display("%0d", $random);
	end
endmodule
