const ta  = document.getElementById('assemblyTextarea');
const out = document.getElementById('out');
 
const INDENT = '    ';
 
/* ---------- token tables ---------- */
 
const REGS = (() => {
  const r = [];
  for (let i = 0; i < 32; i++) { r.push('x' + i); r.push('f' + i); }
  r.push(
    'zero','ra','sp','gp','tp','fp',
    't0','t1','t2','t3','t4','t5','t6',
    's0','s1','s2','s3','s4','s5','s6','s7','s8','s9','s10','s11',
    'a0','a1','a2','a3','a4','a5','a6','a7',
    'ft0','ft1','ft2','ft3','ft4','ft5','ft6','ft7','ft8','ft9','ft10','ft11',
    'fs0','fs1','fs2','fs3','fs4','fs5','fs6','fs7','fs8','fs9','fs10','fs11',
    'fa0','fa1','fa2','fa3','fa4','fa5','fa6','fa7',
    // a few common CSRs
    'mstatus','mtvec','mepc','mcause','mie','mip','mscratch','mhartid','misa',
    'satp','sstatus','stvec','sepc','scause','sie','sip','sscratch'
  );
  return r.sort((x, y) => y.length - x.length);   // longest-first: s11 before s1
})();
 
const INSTR = `
  lui auipc jal jalr
  beq bne blt bge bltu bgeu
  lb lh lw lbu lhu sb sh sw
  addi slti sltiu xori ori andi slli srli srai
  add sub sll slt sltu xor srl sra or and
  fence fence.i ecall ebreak
  csrrw csrrs csrrc csrrwi csrrsi csrrci
  lwu ld sd addiw slliw srliw sraiw addw subw sllw srlw sraw
  mul mulh mulhsu mulhu div divu rem remu
  mulw divw divuw remw remuw
  lr.w sc.w amoswap.w amoadd.w amoxor.w amoand.w amoor.w amomin.w amomax.w amominu.w amomaxu.w
  lr.d sc.d amoswap.d amoadd.d amoxor.d amoand.d amoor.d amomin.d amomax.d amominu.d amomaxu.d
  flw fsw fld fsd
  fmadd.s fmsub.s fnmsub.s fnmadd.s fadd.s fsub.s fmul.s fdiv.s fsqrt.s
  fsgnj.s fsgnjn.s fsgnjx.s fmin.s fmax.s feq.s flt.s fle.s fclass.s
  fcvt.w.s fcvt.wu.s fcvt.s.w fcvt.s.wu fmv.x.w fmv.w.x
  fadd.d fsub.d fmul.d fdiv.d fsqrt.d fcvt.s.d fcvt.d.s
  nop li la mv not neg negw sext.w seqz snez sltz sgtz
  beqz bnez blez bgez bltz bgtz bgt ble bgtu bleu
  j jr ret call tail
  csrr csrw csrs csrc csrwi csrsi csrci
  rdcycle rdtime rdinstret
  mret sret wfi sfence.vma
`.trim().split(/\s+/).sort((x, y) => y.length - x.length);  // fence.i before fence
 
/* ---------- highlighting ---------- */
 
const esc   = s => s.replace(/[&<>]/g, c => ({ '&':'&amp;', '<':'&lt;', '>':'&gt;' }[c]));
const reEsc = s => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
 
const RULES = new RegExp(
    '(#[^\\n]*|\\/\\/[^\\n]*|\\/\\*[\\s\\S]*?\\*\\/)'   // 1 comment
  + '|("(?:[^"\\\\\\n]|\\\\.)*")'                        // 2 string
  + '|((?:\\d+|[.A-Za-z_$][\\w$.]*):)'                   // 3 label
  + '|(\\.[A-Za-z_][\\w.]*)'                             // 4 directive
  + '|\\b(' + REGS.map(reEsc).join('|') + ')\\b'         // 5 register
  + '|\\b(' + INSTR.map(reEsc).join('|') + ')\\b'        // 6 instruction
  + '|\\b(0[xX][0-9a-fA-F]+|\\d+)\\b'                    // 7 number
, 'g');
 
export function render() {
  // trailing newline so the last empty line still has height
  out.innerHTML = esc(ta.value + '\n').replace(RULES,
    (m, c, s, l, d, r, ins, n) => {
      const cls = c ? 'c' : s ? 's' : l ? 'l' : d ? 'd' : r ? 'r' : ins ? 'i' : n ? 'n' : '';
      return cls ? `<span class="${cls}">${m}</span>` : m;
    });
}
 
/* ---------- editing primitives ---------- */
 
// Insert via execCommand so the browser's native undo stack survives.
function insert(text) {
  if (!document.execCommand('insertText', false, text)) {
    const { selectionStart: a, selectionEnd: b, value: v } = ta;
    ta.value = v.slice(0, a) + text + v.slice(b);
    ta.selectionStart = ta.selectionEnd = a + text.length;
    ta.dispatchEvent(new Event('input'));
  }
}
 
// One replacement = one undo step. Selection is restored afterwards.
function replaceRange(from, to, text, selA, selB = selA) {
  ta.setSelectionRange(from, to);
  insert(text);
  if (selA !== undefined) ta.setSelectionRange(selA, selB);
}
 
const lineStartAt = (v, i) => v.lastIndexOf('\n', i - 1) + 1;
const lineEndAt   = (v, i) => (v.indexOf('\n', i) + 1 || v.length + 1) - 1;
const indentOf    = line => line.match(/^[ \t]*/)[0];
 
/* ---------- block indent / outdent ---------- */
 
const OUTDENT_RE = new RegExp('^(\\t| {1,' + INDENT.length + '})');
 
function shiftLines(dir) {
  const v = ta.value;
  const a = ta.selectionStart;
  let   b = ta.selectionEnd;
  if (b > a && v[b - 1] === '\n') b--;   // don't drag in the line after the selection
 
  const from  = lineStartAt(v, a);
  const to    = lineEndAt(v, b);
  const lines = v.slice(from, to).split('\n');
 
  let firstDelta = 0, total = 0;
  const shifted = lines.map((line, i) => {
    let delta = 0;
    if (dir > 0) {
      if (line.length || lines.length === 1) { line = INDENT + line; delta = INDENT.length; }
    } else {
      const m = line.match(OUTDENT_RE);
      if (m) { line = line.slice(m[0].length); delta = -m[0].length; }
    }
    if (i === 0) firstDelta = delta;
    total += delta;
    return line;
  });
 
  if (!total) return;
  const newA = Math.max(from, a + firstDelta);
  replaceRange(from, to, shifted.join('\n'), newA, Math.max(newA, ta.selectionEnd + total));
}
 
/* ---------- line-comment toggle (Ctrl/Cmd + /) ---------- */
 
function toggleComment() {
  const v = ta.value, a = ta.selectionStart;
  let b = ta.selectionEnd;
  if (b > a && v[b - 1] === '\n') b--;
 
  const from  = lineStartAt(v, a);
  const to    = lineEndAt(v, b);
  const lines = v.slice(from, to).split('\n');
  const code  = lines.filter(l => l.trim());
  const off   = code.length && code.every(l => /^\s*#/.test(l));
 
  const next = lines.map(l => {
    if (!l.trim()) return l;
    if (off) return l.replace(/^(\s*)#\s?/, '$1');
    const pad = l.match(/^\s*/)[0];
    return pad + '# ' + l.slice(pad.length);
  }).join('\n');
 
  replaceRange(from, to, next, from, from + next.length);
}
 
/* ---------- keymap ---------- */
 
ta.addEventListener('input', render);
 
ta.addEventListener('keydown', e => {
  const { selectionStart: a, selectionEnd: b, value: v } = ta;
  const collapsed  = a === b;
  const from       = lineStartAt(v, a);
  const beforeCare = v.slice(from, a);
  const next       = v[a];
 
  /* Ctrl/Cmd + /  — toggle line comment */
  if ((e.ctrlKey || e.metaKey) && e.key === '/') {
    e.preventDefault(); toggleComment(); return;
  }
 
  /* Tab / Shift+Tab */
  if (e.key === 'Tab') {
    e.preventDefault();
    const multiline = v.slice(a, b).includes('\n');
    if (e.shiftKey || multiline) shiftLines(e.shiftKey ? -1 : 1);
    else insert(INDENT);
    return;
  }
 
  /* Enter: keep indentation; add a level after a label line */
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    const indent = indentOf(beforeCare);
    const label  = /:$/.test(beforeCare.trimEnd());
    insert('\n' + indent + (label ? INDENT : ''));
    return;
  }
 
  /* Quotes (for .string / .asciz): open a pair or step over the closer */
  if (collapsed && e.key === '"') {
    e.preventDefault();
    if (next === '"') ta.setSelectionRange(a + 1, a + 1);
    else { insert('""'); ta.setSelectionRange(a + 1, a + 1); }
    return;
  }
 
  /* Parentheses for memory operands like 0(sp) */
  if (collapsed && e.key === '(' && !/[\w$]/.test(next || '')) {
    e.preventDefault(); insert('()'); ta.setSelectionRange(a + 1, a + 1); return;
  }
  if (collapsed && e.key === ')' && next === ')') {
    e.preventDefault(); ta.setSelectionRange(a + 1, a + 1); return;
  }
 
  /* Backspace: eat a whole indent unit, or an empty pair */
  if (collapsed && e.key === 'Backspace') {
    if (beforeCare.length && /^[ \t]+$/.test(beforeCare)) {
      e.preventDefault();
      const n = beforeCare.length % INDENT.length || INDENT.length;
      replaceRange(a - n, a, '');
      return;
    }
    const prev = v[a - 1];
    if ((prev === '(' && next === ')') || (prev === '"' && next === '"')) {
      e.preventDefault();
      replaceRange(a - 1, a + 1, '');
    }
  }
});
