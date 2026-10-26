module tb;
    reg [31:0]  r;
    reg [7:0]   mem [0:3];
    reg [191:0] w;

    initial begin
        r = 32'h0;
        $vogls_assert_eq($vogls_drive(r[$vogls_blackbox(8) +: 8], 8'hAB), 8'hAB);
        $vogls_assert_eq(r, 32'h0000_AB00);

        $vogls_assert_eq($vogls_drive(r[$vogls_blackbox(16) +: 8], 8'hCD), 8'hCD);
        $vogls_assert_eq(r, 32'h00CD_AB00);

        $vogls_assert_eq($vogls_drive(r[$vogls_blackbox(16) +: 8], 8'hCD), 8'h00);
        $vogls_assert_eq(r, 32'h00CD_AB00);

        r = 32'h0;
        $vogls_assert_eq($vogls_drive(r[$vogls_blackbox(12) +: 8], 8'hFF), 8'hFF);
        $vogls_assert_eq(r, 32'h000F_F000);

        mem[0] = 8'h00;
        mem[1] = 8'h00;
        mem[2] = 8'h00;
        mem[3] = 8'h00;
        $vogls_assert_eq($vogls_drive(mem[1], 8'h3C), 8'h3C);
        $vogls_assert_eq(mem[1], 8'h3C);

        $vogls_assert_eq($vogls_drive(mem[$vogls_blackbox(2)], 8'h5A), 8'h5A);
        $vogls_assert_eq(mem[0], 8'h00);
        $vogls_assert_eq(mem[1], 8'h3C);
        $vogls_assert_eq(mem[2], 8'h5A);
        $vogls_assert_eq(mem[3], 8'h00);

        r = 32'h0;
        $vogls_assert_eq($vogls_drive(r[$vogls_blackbox(28) +: 8], 8'hFF), 8'h0F);
        $vogls_assert_eq(r, 32'hF000_0000);

        r = 32'h0;
        $vogls_assert_eq($vogls_drive(r[$vogls_blackbox(100) +: 8], 8'hFF), 8'h00);
        $vogls_assert_eq(r, 32'h0000_0000);

        $vogls_assert_eq($vogls_drive(r[$vogls_blackbox(-4) +: 8], 8'hFF), 8'h00);
        $vogls_assert_eq(r, 32'h0000_0000);

        w = 0;
        $vogls_assert_eq($vogls_drive(w[$vogls_blackbox(8) +: 8], 8'hAB), 8'hAB);
        $vogls_assert_eq(w, 192'h0000000000000000_0000000000000000_000000000000ab00);

        w = 0;
        $vogls_assert_eq($vogls_drive(w[$vogls_blackbox(60) +: 8], 8'hFF), 8'hFF);
        $vogls_assert_eq(w, 192'h0000000000000000_000000000000000f_f000000000000000);

        w = 0;
        $vogls_assert_eq(
            $vogls_drive(w[$vogls_blackbox(64) +: 64], 64'hDEADBEEFCAFEBABE),
            64'hDEADBEEFCAFEBABE);
        $vogls_assert_eq(w, 192'h0000000000000000_deadbeefcafebabe_0000000000000000);

        w = 0;
        $vogls_assert_eq(
            $vogls_drive(w[$vogls_blackbox(32) +: 64], 64'h1122334455667788),
            64'h1122334455667788);
        $vogls_assert_eq(w, 192'h0000000000000000_0000000011223344_5566778800000000);

        w = 0;
        $vogls_assert_eq(
            $vogls_drive(w[$vogls_blackbox(0) +: 128], {64'hAAAAAAAAAAAAAAAA, 64'h5555555555555555}),
            128'haaaaaaaaaaaaaaaa_5555555555555555);
        $vogls_assert_eq(w, 192'h0000000000000000_aaaaaaaaaaaaaaaa_5555555555555555);

        w = 0;
        $vogls_assert_eq(
            $vogls_drive(w[$vogls_blackbox(32) +: 128], {64'hAAAAAAAAAAAAAAAA, 64'h5555555555555555}),
            128'haaaaaaaaaaaaaaaa_5555555555555555);
        $vogls_assert_eq(w, 192'h00000000aaaaaaaa_aaaaaaaa55555555_5555555500000000);

        w = 0;
        $vogls_assert_eq(
            $vogls_drive(w[$vogls_blackbox(128) +: 64], 64'hFFFFFFFFFFFFFFFF),
            64'hFFFFFFFFFFFFFFFF);
        $vogls_assert_eq(w, 192'hffffffffffffffff_0000000000000000_0000000000000000);

        w = 0;
        $vogls_assert_eq(
            $vogls_drive(w[$vogls_blackbox(160) +: 64], 64'hFFFFFFFFFFFFFFFF),
            64'h00000000ffffffff);
        $vogls_assert_eq(w, 192'hffffffff00000000_0000000000000000_0000000000000000);

        w = 0;
        $vogls_assert_eq(
            $vogls_drive(w[$vogls_blackbox(120) +: 128], {64'hAAAAAAAAAAAAAAAA, 64'h5555555555555555}),
            128'h00000000000000aa_5555555555555555);
        $vogls_assert_eq(w, 192'haa55555555555555_5500000000000000_0000000000000000);

        w = 0;
        $vogls_assert_eq(
            $vogls_drive(w[$vogls_blackbox(190) +: 64], 64'hFFFFFFFFFFFFFFFF),
            64'h0000000000000003);
        $vogls_assert_eq(w, 192'hc000000000000000_0000000000000000_0000000000000000);

        w = 0;
        $vogls_assert_eq(
            $vogls_drive(w[$vogls_blackbox(128) +: 128], {64'hAAAAAAAAAAAAAAAA, 64'h5555555555555555}),
            128'h0000000000000000_5555555555555555);
        $vogls_assert_eq(w, 192'h5555555555555555_0000000000000000_0000000000000000);

        w = 0;
        $vogls_assert_eq($vogls_drive(w[$vogls_blackbox(256) +: 64], 64'hFFFFFFFFFFFFFFFF), 64'h0);
        $vogls_assert_eq(w, 192'h0);

        $vogls_assert_eq($vogls_drive(w[$vogls_blackbox(-4) +: 8], 8'hFF), 8'h00);
        $vogls_assert_eq(w, 192'h0);

        $vogls_assert_eq(
            $vogls_drive(w[$vogls_blackbox(-32) +: 128], {64'hAAAAAAAAAAAAAAAA, 64'h5555555555555555}),
            128'h0);
        $vogls_assert_eq(w, 192'h0);

        $vogls_assert_eq(
            $vogls_drive(w[$vogls_blackbox(-200) +: 128], {64'hAAAAAAAAAAAAAAAA, 64'h5555555555555555}),
            128'h0);
        $vogls_assert_eq(w, 192'h0);
    end
endmodule
