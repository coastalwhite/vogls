// vogls: verify-stdout
module tb;
initial begin
	$display("%f", 42.1337);
    $display("%e", 42.1337);
    $display("%g", 42.1337);
    $display("%g", 4200000);
end
endmodule
