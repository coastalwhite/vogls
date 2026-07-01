module x();
    initial begin
        $vogls_assert_eq($vogls_posedge(1'b0, 1'b0), 1'b0);
        $vogls_assert_eq($vogls_posedge(1'b1, 1'b0), 1'b0);
        $vogls_assert_eq($vogls_posedge(1'b0, 1'b1), 1'b1);
        $vogls_assert_eq($vogls_posedge(1'b1, 1'b1), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        $vogls_assert_eq($vogls_posedge(1'bx, 1'b0), 1'b0);
        $vogls_assert_eq($vogls_posedge(1'bz, 1'b0), 1'b0);
        $vogls_assert_eq($vogls_posedge(1'bx, 1'b1), 1'b1);
        $vogls_assert_eq($vogls_posedge(1'bz, 1'b1), 1'b1);
        $vogls_assert_eq($vogls_posedge(1'b0, 1'bx), 1'b1);
        $vogls_assert_eq($vogls_posedge(1'b1, 1'bx), 1'b0);
        $vogls_assert_eq($vogls_posedge(1'bz, 1'bx), 1'b0);
        $vogls_assert_eq($vogls_posedge(1'bx, 1'bx), 1'b0);
        $vogls_assert_eq($vogls_posedge(1'b0, 1'bz), 1'b1);
        $vogls_assert_eq($vogls_posedge(1'b1, 1'bz), 1'b0);
        $vogls_assert_eq($vogls_posedge(1'bz, 1'bz), 1'b0);
        $vogls_assert_eq($vogls_posedge(1'bx, 1'bz), 1'b0);
`endif

        $vogls_assert_eq($vogls_negedge(1'b0, 1'b0), 1'b0);
        $vogls_assert_eq($vogls_negedge(1'b1, 1'b0), 1'b1);
        $vogls_assert_eq($vogls_negedge(1'b0, 1'b1), 1'b0);
        $vogls_assert_eq($vogls_negedge(1'b1, 1'b1), 1'b0);
`ifndef __VOGLS__TWO_VALUE_LOGIC
        $vogls_assert_eq($vogls_negedge(1'bx, 1'b0), 1'b1);
        $vogls_assert_eq($vogls_negedge(1'bz, 1'b0), 1'b1);
        $vogls_assert_eq($vogls_negedge(1'bx, 1'b1), 1'b0);
        $vogls_assert_eq($vogls_negedge(1'bz, 1'b1), 1'b0);
        $vogls_assert_eq($vogls_negedge(1'b0, 1'bx), 1'b0);
        $vogls_assert_eq($vogls_negedge(1'b1, 1'bx), 1'b1);
        $vogls_assert_eq($vogls_negedge(1'bz, 1'bx), 1'b0);
        $vogls_assert_eq($vogls_negedge(1'bx, 1'bx), 1'b0);
        $vogls_assert_eq($vogls_negedge(1'b0, 1'bz), 1'b0);
        $vogls_assert_eq($vogls_negedge(1'b1, 1'bz), 1'b1);
        $vogls_assert_eq($vogls_negedge(1'bz, 1'bz), 1'b0);
        $vogls_assert_eq($vogls_negedge(1'bx, 1'bz), 1'b0);
`endif
    end
endmodule
