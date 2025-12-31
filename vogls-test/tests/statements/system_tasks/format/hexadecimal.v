// vogls: verify-stdout
module x();
	initial begin
        $display("%h", 12'h42);   // 042
        $display("%0h", 12'h42);  // 42
        $display("%03h", 12'h42); // 042
        $display("%02h", 12'h42); // 42
        $display("%03h", 12'h2);  // 002
	end
endmodule
