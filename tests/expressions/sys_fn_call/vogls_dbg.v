// vogls: verify-stdout
module x();
    reg a;
    initial a = $vogls_dbg(42);
endmodule
