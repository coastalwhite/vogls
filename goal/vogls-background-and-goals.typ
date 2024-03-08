#set page(paper: "a4")

#align(center, [
  #image("./assets/logo.svg", width: 60%)
  #v(0mm)
  #text(size: 1.5em)[VLSI Open Gate-Level Simulator]
])

#v(1fr)

#align(center)[
  #text(size: 2.5em)[*Research Project Proposal*]
]

#v(3cm)

#grid(
  columns: (1fr, 1fr),

  align(left,  [ _Author_: Gijs Burghoorn ]),
  align(right, [ _Date_: March, 2024      ]),
)

#counter(page).update(0)
#set page(numbering: "1")
#pagebreak()

#align(center, [
  #set par(justify: true)

  = Abstract

  Lorem ipsum dolor sit amet, officia excepteur ex fugiat reprehenderit enim
  labore culpa sint ad nisi Lorem pariatur mollit ex esse exercitation amet.
  Nisi anim cupidatat excepteur officia. Reprehenderit nostrud nostrud ipsum
  Lorem est aliquip amet voluptate voluptate dolor minim nulla est proident.
  Nostrud officia pariatur ut officia. Sit irure elit esse ea nulla sunt ex
  occaecat reprehenderit commodo officia dolor Lorem duis laboris cupidatat
  officia voluptate. Culpa proident adipisicing id nulla nisi laboris ex in
  Lorem sunt duis officia eiusmod. Aliqua reprehenderit commodo ex non excepteur
  duis sunt velit enim. Voluptate laboris sint cupidatat ullamco ut ea
  consectetur et est culpa et culpa duis.
])

#pagebreak()

= Outline

#outline(title: none)

#pagebreak()

#set par(justify: true)
#counter(heading).update(0)
#set heading(numbering: "1.")

= Introduction

== Challenges in Digital Design
=== Security Challenges in Digital Design
=== Robustness Challenges in Digital Design
=== Cost Challenges in Digital Design
=== Power-efficiency Challenges in Digital Design

== Electronic Design Automation
=== Synthesis steps
=== Verification steps
=== Static Timing Analysis

== Gate-Level Simulation
=== Gate-Level Simulation Timing Models
=== Gate-Level Simulation Evaluation Strategies

= Literature Review

== Existing Solutions
== Event-Driven Simulation
== Compiler-Driven Simulation

= Methodology

Goals: be a Verilator for GLS

- Design a industry usable Gate-Level Simulator
  - Allow programmatic use for integration into other tools
  - Reduce barrier to entry to do Gate-Level Simulation
  - Preserve knowledge
  - Use a mix of compiled and event-driven simulation for models
    - Zero-delay
    - Unit-delay
    - Full-Timing
- Create good documentation on how to do basic things

= Evaluation

= Expected Contributions

= Timeline

= Budget

= Conclusion

= References