module x();
    initial begin
`ifdef X
        $vogls_assert_eq(0, 1);
`endif

`ifdef X
        $vogls_assert_eq(0, 1);
`elsif Y
        $vogls_assert_eq(0, 1);
`endif

`ifdef X
        $vogls_assert_eq(0, 1);
`elsif Y
        $vogls_assert_eq(0, 1);
`else
        $vogls_assert_eq(1, 1);
`endif

`ifdef X
    `define Z 0
`elsif Y
    `define Z 0
`else
    `define Z 1
`endif
        $vogls_assert_eq(`Z, 1);

`ifdef X
    `ifdef Z
        $vogls_assert_eq(0, 1);
    `endif
`endif

`ifdef X
    `ifdef Z
        $vogls_assert_eq(0, 1);
    `elsif Z
        $vogls_assert_eq(0, 1);
    `else
        $vogls_assert_eq(0, 1);
    `endif
`endif
    end
endmodule


