module tb;
    reg [31:0]  r;
    reg [191:0] w;

    initial begin
        r = 32'h0;
        $vogls_assert_eq($vogls_drive(r, 32'hFFFF_FFFF), 32'hFFFF_FFFF);
        $vogls_assert_eq(r, 32'hFFFF_FFFF);

        r = 32'h0;
        $vogls_assert_eq($vogls_drive(r[15:8], 8'hFF), 8'hFF);
        $vogls_assert_eq(r, 32'h0000_FF00);

        $vogls_assert_eq($vogls_drive(r[0], 1'b1), 1'b1);
        $vogls_assert_eq(r, 32'h0000_FF01);

        $vogls_assert_eq($vogls_drive(r[15:8], 8'hFF), 8'h00);
        $vogls_assert_eq(r, 32'h0000_FF01);

        $vogls_assert_eq($vogls_drive(r[15:8], 8'hF0), 8'h0F);
        $vogls_assert_eq(r, 32'h0000_F001);

        r = 32'h0;
        $vogls_assert_eq($vogls_drive(r[31:24], 8'h5A), 8'h5A);
        $vogls_assert_eq(r, 32'h5A00_0000);

        r = 32'h0;
        $vogls_assert_eq($vogls_drive(r[28 +: 8], 8'hFF), 8'h0F);
        $vogls_assert_eq(r, 32'hF000_0000);

        r = 32'h0;
        $vogls_assert_eq($vogls_drive(r[35:28], 8'hFF), 8'h0F);
        $vogls_assert_eq(r, 32'hF000_0000);

        r = 32'h0;
        $vogls_assert_eq($vogls_drive(r[100 +: 8], 8'hFF), 8'h00);
        $vogls_assert_eq(r, 32'h0000_0000);

        w = 0;
        $vogls_assert_eq($vogls_drive(w[8 +: 8], 8'hAB), 8'hAB);
        $vogls_assert_eq(w, 192'h0000000000000000_0000000000000000_000000000000ab00);

        w = 0;
        $vogls_assert_eq($vogls_drive(w[60 +: 8], 8'hFF), 8'hFF);
        $vogls_assert_eq(w, 192'h0000000000000000_000000000000000f_f000000000000000);

        w = 0;
        $vogls_assert_eq($vogls_drive(w[64 +: 64], 64'hDEADBEEFCAFEBABE), 64'hDEADBEEFCAFEBABE);
        $vogls_assert_eq(w, 192'h0000000000000000_deadbeefcafebabe_0000000000000000);

        w = 0;
        $vogls_assert_eq($vogls_drive(w[32 +: 64], 64'h1122334455667788), 64'h1122334455667788);
        $vogls_assert_eq(w, 192'h0000000000000000_0000000011223344_5566778800000000);

        w = 0;
        $vogls_assert_eq(
            $vogls_drive(w[0 +: 128], {64'hAAAAAAAAAAAAAAAA, 64'h5555555555555555}),
            128'haaaaaaaaaaaaaaaa_5555555555555555);
        $vogls_assert_eq(w, 192'h0000000000000000_aaaaaaaaaaaaaaaa_5555555555555555);

        w = 0;
        $vogls_assert_eq(
            $vogls_drive(w[32 +: 128], {64'hAAAAAAAAAAAAAAAA, 64'h5555555555555555}),
            128'haaaaaaaaaaaaaaaa_5555555555555555);
        $vogls_assert_eq(w, 192'h00000000aaaaaaaa_aaaaaaaa55555555_5555555500000000);

        w = 0;
        $vogls_assert_eq($vogls_drive(w[128 +: 64], 64'hFFFFFFFFFFFFFFFF), 64'hFFFFFFFFFFFFFFFF);
        $vogls_assert_eq(w, 192'hffffffffffffffff_0000000000000000_0000000000000000);

        w = 0;
        $vogls_assert_eq($vogls_drive(w[160 +: 64], 64'hFFFFFFFFFFFFFFFF), 64'h00000000ffffffff);
        $vogls_assert_eq(w, 192'hffffffff00000000_0000000000000000_0000000000000000);

        w = 0;
        $vogls_assert_eq($vogls_drive(w[190 +: 64], 64'hFFFFFFFFFFFFFFFF), 64'h0000000000000003);
        $vogls_assert_eq(w, 192'hc000000000000000_0000000000000000_0000000000000000);

        w = 0;
        $vogls_assert_eq(
            $vogls_drive(w[128 +: 128], {64'hAAAAAAAAAAAAAAAA, 64'h5555555555555555}),
            128'h0000000000000000_5555555555555555);
        $vogls_assert_eq(w, 192'h5555555555555555_0000000000000000_0000000000000000);

        w = 0;
        $vogls_assert_eq($vogls_drive(w[256 +: 64], 64'hFFFFFFFFFFFFFFFF), 64'h0);
        $vogls_assert_eq(w, 192'h0);

        $vogls_assert_eq($vogls_drive(w[-4 +: 8], 8'hFF), 8'h00);
        $vogls_assert_eq(w, 192'h0);

        $vogls_assert_eq(
            $vogls_drive(w[-32 +: 128], {64'hAAAAAAAAAAAAAAAA, 64'h5555555555555555}),
            128'h0);
        $vogls_assert_eq(w, 192'h0);

        $vogls_assert_eq(
            $vogls_drive(w[-200 +: 128], {64'hAAAAAAAAAAAAAAAA, 64'h5555555555555555}),
            128'h0);
        $vogls_assert_eq(w, 192'h0);
    end
endmodule
