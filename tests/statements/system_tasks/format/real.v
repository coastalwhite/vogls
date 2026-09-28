// vogls: verify-stdout
module tb;
initial begin
	$display("%f", 42.1337);
    $display("%e", 42.1337);
    $display("%g", 42.1337);
    $display("%g", 4200000);
    $display("%%f = %f", 1.5);
    $display("%%f = %f", 256);
    $display("%%g = %g", 1.5);
    $display("%%e = %e", 1.5);
    $display("%%.2f = %.2f", 1.5);
    $display("%%10.3f = %10.3f", 1.5);
    $display("int %%f = %f", 1);
    $display("int %%e = %e", 1);
    $display("int %%g = %g", 1);
end
endmodule
