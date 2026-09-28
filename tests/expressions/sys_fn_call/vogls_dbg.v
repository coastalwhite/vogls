// vogls: verify-stdout
module x();
    wire a;
    initial a = $vogls_dbg(42);
endmodule
