// vogls: verify-stdout
module x();
	integer i;
    task y; begin
        $display("Hello!");
        #1
        i = 42;
        $display("Bye!");
    end endtask

	initial begin
        i = 1337;
        y;
        $vogls_assert_eq(i, 42);
    end
endmodule
