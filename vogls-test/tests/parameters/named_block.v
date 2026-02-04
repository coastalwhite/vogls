// vogls: verify-stdout
module x();
    initial begin: named_block
        parameter integer X = 42;
        $display("%0d", X);
    end
endmodule
