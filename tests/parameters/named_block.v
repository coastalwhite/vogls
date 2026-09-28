// vogls: verify-stdout
module x();
    initial begin: named_block
        parameter X = 42;
        $display("%0d", X);
    end
endmodule
