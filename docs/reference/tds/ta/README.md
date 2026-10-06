<div align="center">

  <img src="./ta.png" width="100%" alt="Dark navy Hyprland-style wallpaper with an electric cyan mark">

  <h1>WA 09 Telecommunications Administrator</h1>

  <p><strong>NEC lookup maps, electrical theory, exam strategy, and study resources</strong></p>

  <p>
    <img alt="WA 09" src="https://img.shields.io/badge/EXAM-WA%2009-00B8F0?style=for-the-badge&labelColor=071426">
    <img alt="Open book" src="https://img.shields.io/badge/FORMAT-OPEN%20BOOK-00DDD0?style=for-the-badge&labelColor=071426">
    <img alt="NEC 2023 study map" src="https://img.shields.io/badge/STUDY%20MAP-NEC%202023-0B6FA4?style=for-the-badge&labelColor=071426">
    <img alt="Washington law" src="https://img.shields.io/badge/LAW-RCW%20%2B%20WAC-12304A?style=for-the-badge&labelColor=071426">
  </p>

  <sub>Designed around the midnight navy and electric cyan palette of <code>ta.png</code>.</sub>

</div>

> [!IMPORTANT]
> The study maps in this repository follow the **2023 NEC organization**, including Articles 722, 724, and 726. The linked PSI Candidate Information Bulletin dated October 30, 2025 identifies the **2020 NEC** as the exam reference. Washington's adopted installation code and PSI's exam reference can change on different schedules. Confirm your exam edition before test day and do not mix editions during timed lookup practice.

<p align="center">
  <a href="#start-here">Start here</a> ·
  <a href="#exam-blueprint">Exam blueprint</a> ·
  <a href="#chapter-3">Chapter 3</a> ·
  <a href="#chapter-7">Chapter 7</a> ·
  <a href="#chapter-8">Chapter 8</a> ·
  <a href="#theory">Theory</a> ·
  <a href="#practice">Practice</a> ·
  <a href="#resources">Resources</a> ·
  <a href="#anki">Anki</a>
</p>

---

<a id="start-here"></a>

<details open>
<summary><strong>01 / Start Here</strong></summary>

### Purpose

This guide is a single lookup-oriented study hub for the Washington **09-Telecommunications Administrator** examination. It combines:

- the official exam weighting and timing;
- Chapter 3, Chapter 7, and Chapter 8 article maps;
- high-yield electrical and telecommunications theory;
- ampere-turn practice problems;
- official code, law, testing, and study links; and
- an Anki workflow for retaining lookup paths.

The goal is not to memorize the entire NEC. The goal is to recognize the wording in a question, identify the correct code neighborhood, and verify the exact requirement efficiently.

<details>
<summary><strong>Recommended study sequence</strong></summary>

1. Read the exam blueprint and mark the highest-weight domains.
2. Memorize the three navigation maps: `CABFER`, `ESPOWER`, and `ANTENNA`.
3. Practice locating answers in your permitted exam references.
4. Drill the theory formulas without the book.
5. Use timed mixed practice to switch quickly between code lookup and calculation.
6. Review RCW 19.28 and WAC 296-46B as a separate scored section.

</details>

<details>
<summary><strong>How to read this guide</strong></summary>

| Element | Meaning |
|---|---|
| `Monospace text` | A mnemonic, search term, or scratch-sheet shorthand |
| **Bold text** | A high-value trigger word or exam distinction |
| Tables | Fast comparisons and article maps |
| Displayed equations | General trade theory to calculate without an NEC lookup |
| Collapsible panels | Material grouped for focused review |

> [!TIP]
> Build a two-step habit: **keyword -> article**, then **article -> exact section or table**. Do not stop at a remembered summary when the test expects the book's wording.

</details>

</details>

---

<a id="exam-blueprint"></a>

<details>
<summary><strong>02 / Official Exam Blueprint</strong></summary>

<details open>
<summary><strong>Exam structure</strong></summary>

The PSI bulletin describes two separately scored, open-book sections. A score of **70% or greater** is required on each section.

| Section | Time allowed | Questions | Passing score |
|---|---:|---:|---:|
| NEC and Theory | 2 hours | 30 | 70% or greater |
| Washington Codes | 1 hour | 17 | 70% or greater |
| **Total appointment** | **3 hours** | **47** | **Pass both sections** |

Official source: [Washington Electrical Certification Candidate Information Bulletin](https://lni.wa.gov/licensing-permits/_docs/display_bulletin.pdf)

</details>

<details>
<summary><strong>NEC and Theory question distribution</strong></summary>

Question counts are approximate. A dash means the subject remains in scope but the bulletin does not assign it a separate expected count.

| Article range | Knowledge area | Approximate questions |
|---|---|---:|
| NEC 90 | Introduction | 2 |
| NEC 100–110 | General Requirements | 3 |
| NEC 200–225, 280–285 | Wiring and Protection | — |
| NEC 230 | Services | — |
| NEC 240 | Overcurrent Protection | — |
| NEC 250 | Grounding and Bonding | 3 |
| NEC 300–398 | Wiring Methods | 3 |
| NEC 400–427, 455–480 | Electrical Equipment | — |
| NEC 430–450 | Motors, HVAC, Generators, and Transformers | — |
| NEC 490 | Equipment over 600 V | — |
| NEC 500–516 | Hazardous Locations | 1 |
| NEC 517–590 | Special Occupancies | — |
| NEC 600–695 | Special Equipment | — |
| NEC 700–702 | Emergency and Standby Systems | — |
| NEC 705–780 | Special Conditions | 4 |
| **NEC 800–830** | **Communication Systems** | **8** |
| General trade knowledge | Theory and calculations | 6 |
| | **Total** | **30** |

> [!IMPORTANT]
> Communication Systems is the largest NEC block: **8 of 30** NEC and Theory questions. Washington Laws and Rules is an additional **17-question section** and must be passed separately.

</details>

<details>
<summary><strong>Study priority</strong></summary>

| Priority | Domain | Questions | Reason |
|---:|---|---:|---|
| 1 | RCW 19.28 and WAC 296-46B | 17 | Largest separately scored body of material |
| 1 | NEC 800–830 Communication Systems | 8 | Telecommunications core |
| 2 | General Trade Knowledge and Theory | 6 | Must be calculated or reasoned quickly |
| 2 | NEC 705–780 Special Conditions | 4 | Power-limited, fire alarm, and optical-fiber material |
| 3 | NEC 100–110, 250, and 300–398 | 3 each | Foundational definitions, grounding, and wiring methods |
| 4 | NEC 90 and 500–516 | 2 and 1 | Lower count but direct lookup opportunities |
| 5 | Remaining listed ranges | Low weight | Still in scope and may appear in mixed questions |

</details>

<details>
<summary><strong>Exam-room constraints worth practicing</strong></summary>

Based on the linked bulletin:

- the examinations are open book;
- approved references may be highlighted, underlined, and permanently tabbed;
- removable notes are not permitted;
- references may not be written in;
- a silent, nonprinting, nonprogrammable calculator is permitted; and
- code, law, rule, and theory knowledge are all expected.

> [!WARNING]
> Always use the current bulletin supplied for your appointment. Testing rules, approved materials, and code editions can change.

</details>

</details>

---

<a id="chapter-3"></a>

<details>
<summary><strong>03 / Chapter 3 Wiring Methods — CABFER Map</strong></summary>

<details open>
<summary><strong>One-word map</strong></summary>

**CABFER** divides the Chapter 3 article numbers into memorable bands:

| Letter | Range | Band |
|:---:|---:|---|
| C | 300–315 | Core rules, conductors, cabinets, and boxes |
| A | 320–340 | Armored and named cable wiring methods |
| B | 342–362 | Conduit and tubing |
| F | 366–371 | Feeders by bus, gutters, and cablebus |
| E | 372–384 | Embedded floor raceways, wireways, extensions, and strut raceways |
| R | 386–398 | Raceways, trays, and legacy/open wiring |

```text
CABFER: C 300–315 | A 320–340 | B 342–362 | F 366–371 | E 372–384 | R 386–398
```

</details>

<details>
<summary><strong>C — Core rules, conductors, cabinets, and boxes</strong></summary>

| Article | Subject |
|---:|---|
| 300 | General Requirements for Wiring Methods and Materials |
| 305 | General Requirements for Wiring Methods and Materials for Systems over 1000 V ac and 1500 V dc |
| 310 | Conductors for General Wiring |
| 312 | Cabinets, Cutout Boxes, and Meter Socket Enclosures |
| 314 | Outlet, Device, Pull, and Junction Boxes; Conduit Bodies; Fittings; Handhole Enclosures |

#### Article 300 section map

| Section | Keyword | Lookup cue |
|---:|---|---|
| 300.1 | Scope | What Article 300 covers |
| 300.2 | Limitations | Limits on applying the article |
| 300.3 | Conductors | General conductor installation rules |
| 300.4 | Damage | Protection against physical damage |
| 300.5 | Underground | Underground installations |
| 300.6 | Corrosion | Protection against corrosion and deterioration |
| 300.7 | Temperature | Raceways exposed to different temperatures |
| 300.8 | Other systems | Conductors installed with other systems |
| 300.9 | Wet locations | Raceways in wet locations above grade |
| 300.10 | Continuity | Electrical continuity of metal raceways, armor, and enclosures |
| 300.11 | Support | Securing and supporting |
| 300.12 | Mechanical continuity | Raceway and cable sheath continuity |
| 300.13 | Conductors and splices | Mechanical and electrical continuity |
| 300.14 | Free length | Free conductor at outlets, junctions, and switch points |
| 300.15 | Boxes and fittings | Where boxes, conduit bodies, or fittings are required |
| 300.16 | Transition | Raceway or cable transition to open or concealed wiring |
| 300.17 | Raceway fill | Number and size of conductors and cables in raceways |
| 300.18 | Raceway installation | Installation completion and arrangement |
| 300.19 | Vertical support | Supporting conductors in vertical raceways |
| 300.20 | Induced currents | Ferrous metal enclosures and raceways |
| 300.21 | Firestopping | Spread of fire or products of combustion |
| 300.22 | Environmental air | Ducts, plenums, and other environmental-air spaces |
| 300.23 | Access panels | Panels designed to allow access |
| 300.25 | Exit enclosures | Stair towers and similar exit enclosures |
| 300.26 | Signal classification | Remote-control and signaling circuit classification |

High-yield sequence:

```text
300.10–300.20
Continuity -> Support -> Sheath -> Splices -> Free Length -> Boxes ->
Transition -> Raceway Fill -> Raceway Install -> Vertical Support -> Induction
```

</details>

<details>
<summary><strong>A — Cable wiring methods</strong></summary>

| Article | Wiring method |
|---:|---|
| 320 | Armored Cable: Type AC |
| 322 | Flat Cable Assemblies: Type FC |
| 324 | Flat Conductor Cable: Type FCC |
| 326 | Integrated Gas Spacer Cable: Type IGS |
| 330 | Metal-Clad Cable: Type MC |
| 332 | Mineral-Insulated, Metal-Sheathed Cable: Type MI |
| 334 | Nonmetallic-Sheathed Cable: Types NM and NMC |
| 335 | Instrumentation Tray Cable: Type ITC |
| 336 | Power and Control Tray Cable: Type TC |
| 337 | Type P Cable |
| 338 | Service-Entrance Cable: Types SE and USE |
| 340 | Underground Feeder and Branch-Circuit Cable: Type UF |

```text
320 AC | 330 MC | 334 NM | 338 SE/USE | 340 UF
```

</details>

<details>
<summary><strong>B — Conduit and tubing</strong></summary>

| Article | Wiring method |
|---:|---|
| 342 | Intermediate Metal Conduit: IMC |
| 344 | Rigid Metal Conduit: RMC |
| 348 | Flexible Metal Conduit: FMC |
| 350 | Liquidtight Flexible Metal Conduit: LFMC |
| 352 | Rigid Polyvinyl Chloride Conduit: PVC |
| 353 | High Density Polyethylene Conduit: HDPE |
| 354 | Nonmetallic Underground Conduit with Conductors: NUCC |
| 355 | Reinforced Thermosetting Resin Conduit: RTRC |
| 356 | Liquidtight Flexible Nonmetallic Conduit: LFNC |
| 358 | Electrical Metallic Tubing: EMT |
| 360 | Flexible Metallic Tubing: FMT |
| 362 | Electrical Nonmetallic Tubing: ENT |

```text
342 IMC | 344 RMC | 352 PVC | 358 EMT | 362 ENT
```

</details>

<details>
<summary><strong>F — Bus systems and gutters</strong></summary>

| Article | Wiring method |
|---:|---|
| 366 | Auxiliary Gutters |
| 368 | Busways |
| 369 | Insulated Bus Pipe and Tubular Covered Conductor Systems |
| 370 | Cablebus |
| 371 | Flexible Bus Systems |

```text
366 Gutters | 368 Busways | 370 Cablebus
```

</details>

<details>
<summary><strong>E — Floor raceways, wireways, extensions, and strut raceways</strong></summary>

| Article | Wiring method |
|---:|---|
| 372 | Cellular Concrete Floor Raceways |
| 374 | Cellular Metal Floor Raceways |
| 376 | Metal Wireways |
| 378 | Nonmetallic Wireways |
| 380 | Multioutlet Assemblies |
| 382 | Nonmetallic Extensions |
| 384 | Strut-Type Channel Raceway |

```text
372/374 Floor | 376/378 Wireways | 380/382/384 Assemblies, Extensions, Strut
```

</details>

<details>
<summary><strong>R — Raceways, trays, and legacy/open wiring</strong></summary>

| Article | Wiring method |
|---:|---|
| 386 | Surface Metal Raceways |
| 388 | Surface Nonmetallic Raceways |
| 390 | Underfloor Raceways |
| 392 | Cable Trays |
| 393 | Low-Voltage Suspended Ceiling Power Distribution Systems |
| 394 | Concealed Knob-and-Tube Wiring |
| 395 | Outdoor Overhead Conductors over 1000 V |
| 396 | Messenger-Supported Wiring |
| 398 | Open Wiring on Insulators |

```text
386/388 Surface | 390 Underfloor | 392 Trays | 394–398 Legacy/Open
```

</details>

<details>
<summary><strong>Common lookup trap</strong></summary>

- **Single conductor**, **ferrous metal**, **induced current**, or **inductive heating** points toward the general wiring-method rule in 300.20.
- **Surface metal raceway as a wiring method** points toward Article 386.
- The word *ferrous* is not enough by itself; identify whether the question concerns induced heating or a named raceway system.

</details>

</details>

---

<a id="chapter-7"></a>

<details>
<summary><strong>04 / Chapter 7 Special Conditions — ESPOWER Map</strong></summary>

<details open>
<summary><strong>Range map</strong></summary>

**ESPOWER** groups the telecommunications-relevant Chapter 7 subjects:

```text
E 700–706 | S 708–710 | P 722–726 | O 728 | W 750 | E 760 | R 770
```

| Letter | Range | Trigger idea |
|:---:|---:|---|
| E | 700–706 | Emergency, standby, interconnected power, and energy storage |
| S | 708–710 | Special operations and stand-alone systems |
| P | 722–726 | Power-limited and fault-managed power |
| O | 728 | Operates during fire: fire-resistive cable systems |
| W | 750 | Watt and load management |
| E | 760 | Emergency signaling: fire alarm systems |
| R | 770 | Remote transmission by light: optical fiber |

</details>

<details>
<summary><strong>E — Emergency, standby, production, and storage</strong></summary>

| Trigger words | Article |
|---|---:|
| Emergency system, egress lighting, life-safety loads | 700 |
| Legally required standby, code-mandated standby loads | 701 |
| Optional standby, owner-selected backup | 702 |
| Interconnected generation, PV, utility-interactive power | 705 |
| Batteries, energy storage system, rack batteries | 706 |

</details>

<details>
<summary><strong>S — Special operations and stand-alone systems</strong></summary>

| Trigger words | Article |
|---|---:|
| Critical operations power system, COPS, mission-critical facility | 708 |
| Stand-alone system, isolated source, not utility-connected | 710 |

</details>

<details>
<summary><strong>P — Power-limited and fault-managed power</strong></summary>

| Trigger words | Article |
|---|---:|
| Cables for power-limited circuits or fault-managed power circuits | 722 |
| Class 1 power-limited, remote-control, or signaling circuit | 724 |
| Class 2 or Class 3 power-limited circuit | 725 |
| Class 4 fault-managed power system | 726 |

</details>

<details>
<summary><strong>O, W, E, R — Fire-resistive cable through optical fiber</strong></summary>

| Trigger words | Article |
|---|---:|
| Circuit integrity, fire-resistive cable, survival during fire exposure | 728 |
| Energy management, load shedding, demand response | 750 |
| Fire alarm circuit, initiating device, notification appliance, SLC or NAC | 760 |
| Optical fiber cable, fiber backbone, transmission by light | 770 |

</details>

<details>
<summary><strong>Chapter 7 scratch-sheet line</strong></summary>

```text
ESPOWER: 700–710 Emergency/Standby/Special Power |
722–728 Power-Limited/Fault-Managed/Fire-Resistive Cable |
750 Energy Management | 760 Fire Alarm | 770 Optical Fiber
```

</details>

</details>

---

<a id="chapter-8"></a>

<details>
<summary><strong>05 / Chapter 8 Communication Systems — ANTENNA Map</strong></summary>

<details open>
<summary><strong>Range map</strong></summary>

Use **ANTENNA** as the subject cue, then remember the three article bands:

```text
A: 800–805 General and Communications Circuits
N: 810 Antenna Systems
T: 820–840 Television, Network-Powered, and Premises-Powered Broadband
```

| Range | Article | Subject |
|---:|---:|---|
| 800–805 | 800 | General Requirements for Communications Systems |
| 800–805 | 805 | Communications Circuits |
| 810 | 810 | Antenna Systems |
| 820–840 | 820 | Community Antenna Television and Radio Distribution Systems |
| 820–840 | 830 | Network-Powered Broadband Communications Systems |
| 820–840 | 840 | Premises-Powered Broadband Communications Systems |

> [!NOTE]
> Chapter 8 generally stands apart from Chapters 1 through 7 except where a Chapter 8 rule specifically references another requirement. Verify the applicable scope and reference chain in the code book.

> [!CAUTION]
> The PSI blueprint expressly lists **NEC 800–830**. Article 840 is retained here as adjacent broadband context from the source guide, but it is not separately named in that examination range.

</details>

<details>
<summary><strong>Article 800 — General communications requirements</strong></summary>

Lookup themes include:

- scope and definitions shared across communications systems;
- grounding and bonding;
- equipment and cable listing;
- protection at the building entrance;
- cable installation and separation; and
- references that apply across later Chapter 8 articles.

High-value cue: when a question describes a rule common to multiple communications technologies, begin with the general communications requirements before jumping to the system-specific article.

</details>

<details>
<summary><strong>Article 805 — Communications circuits</strong></summary>

Lookup themes include communications wiring from the service point to terminal equipment, cable types, building pathways, listing, separation, and installation methods for voice and data circuits.

</details>

<details>
<summary><strong>Article 810 — Antenna systems</strong></summary>

Lookup themes include radio and television receiving equipment, antenna masts, lead-in conductors, grounding, bonding, and satellite receiving systems.

</details>

<details>
<summary><strong>Articles 820, 830, and 840 — Distribution and broadband</strong></summary>

| Question cue | First article to inspect |
|---|---:|
| Coaxial CATV or radio distribution | 820 |
| Network-powered broadband from the network side | 830 |
| Premises-powered broadband or customer-side powered optical network equipment | 840 |

</details>

<details>
<summary><strong>Chapter 8 scratch-sheet line</strong></summary>

```text
ANTENNA: 800 General | 805 Comms Circuits | 810 Antennas |
820 CATV | 830 Network-Powered Broadband | 840 Premises-Powered Broadband
```

</details>

</details>

---

<a id="theory"></a>

<details>
<summary><strong>06 / Electrical and Telecommunications Theory</strong></summary>

<details open>
<summary><strong>Formula index</strong></summary>

| Topic | Formula |
|---|---|
| Ohm's law | $V = IR$ |
| DC power | $P = VI = I^2R = \dfrac{V^2}{R}$ |
| Transformer voltage ratio | $\dfrac{V_s}{V_p}=\dfrac{N_s}{N_p}$ |
| Transformer current ratio | $\dfrac{I_s}{I_p}=\dfrac{N_p}{N_s}$ |
| Ampere-turns | $\mathcal{F}=NI$ |
| Sinusoidal RMS voltage | $V_{\mathrm{rms}}=\dfrac{V_{\mathrm{peak}}}{\sqrt{2}}\approx0.707V_{\mathrm{peak}}$ |
| Capacitive reactance | $X_C=\dfrac{1}{2\pi fC}$ |
| Inductive reactance | $X_L=2\pi fL$ |
| Balanced three-phase real power | $P=\sqrt{3}VI\operatorname{PF}$ |
| Reactive power | $Q=\sqrt{S^2-P^2}$ |
| Power ratio in decibels | $\Delta P_{\mathrm{dB}}=10\log_{10}\left(\dfrac{P_2}{P_1}\right)$ |

</details>

<details>
<summary><strong>Transformer ratios</strong></summary>

For an ideal transformer, voltage follows the turns ratio:

$$
V_s=V_p\left(\frac{N_s}{N_p}\right)
$$

Example:

$$
V_s=240\left(\frac{50}{200}\right)=60\text{ V}
$$

Current varies inversely with turns:

$$
I_s=I_p\left(\frac{N_p}{N_s}\right)
$$

Example:

$$
I_s=5\left(\frac{600}{150}\right)=20\text{ A}
$$

For an ideal transformer, apparent input power equals apparent output power:

$$
V_pI_p=V_sI_s
$$

Example:

$$
I_s=\frac{120\times8}{480}=2\text{ A}
$$

| Symbol | Meaning |
|---|---|
| $V_p$, $V_s$ | Primary and secondary voltage |
| $I_p$, $I_s$ | Primary and secondary current |
| $N_p$, $N_s$ | Primary and secondary turns |

</details>

<details>
<summary><strong>Ampere-turns</strong></summary>

Magnetomotive force in ampere-turns is:

$$
\mathcal{F}=NI
$$

where $N$ is the number of turns and $I$ is current in amperes.

Rearrange as needed:

$$
I=\frac{\mathcal{F}}{N}
\qquad
N=\frac{\mathcal{F}}{I}
$$

</details>

<details>
<summary><strong>RMS, reactance, and phase</strong></summary>

For a sinusoidal waveform:

$$
V_{\mathrm{rms}}=0.707V_{\mathrm{peak}}
$$

Example:

$$
V_{\mathrm{rms}}=170\times0.707\approx120\text{ V}
$$

Capacitive reactance:

$$
X_C=\frac{1}{2\pi fC}
$$

Inductive reactance:

$$
X_L=2\pi fL
$$

| Load | Current relative to voltage | Frequency effect |
|---|---|---|
| Inductive | Current **lags** voltage | $X_L$ increases as frequency increases |
| Capacitive | Current **leads** voltage | $X_C$ decreases as frequency increases |

</details>

<details>
<summary><strong>Power and three-phase systems</strong></summary>

Useful power identities:

$$
P=VI=I^2R=\frac{V^2}{R}
$$

Examples:

$$
P=(4)^2(12)=192\text{ W}
$$

$$
P=\frac{(240)^2}{60}=960\text{ W}
$$

For a balanced three-phase load using line-to-line voltage and line current:

$$
P=\sqrt{3}VI\operatorname{PF}
$$

Example:

$$
P=1.732(480)(10)(1.0)\approx8{,}314\text{ W}
$$

Power triangle relationship:

$$
S^2=P^2+Q^2
$$

Example:

$$
Q=\sqrt{10{,}000^2-8{,}000^2}=6{,}000\text{ VAR}
$$

| Symbol | Meaning |
|---|---|
| $P$ | Real power in watts |
| $Q$ | Reactive power in VAR |
| $S$ | Apparent power in VA |
| $\operatorname{PF}$ | Power factor |

</details>

<details>
<summary><strong>Series, parallel, KCL, and KVL</strong></summary>

Series resistance adds directly:

$$
R_{\mathrm{total}}=R_1+R_2+R_3
$$

Example:

$$
R_{\mathrm{total}}=10+15+25=50\ \Omega
$$

For two parallel branches supplied at 120 V:

$$
I_1=\frac{120}{40}=3\text{ A}
\qquad
I_2=\frac{120}{60}=2\text{ A}
$$

By Kirchhoff's Current Law:

$$
I_{\mathrm{total}}=I_1+I_2=5\text{ A}
$$

| Law | Working statement |
|---|---|
| KCL | The sum of currents entering a node equals the sum leaving it. |
| KVL | The algebraic sum of voltage rises and drops around a closed loop is zero. |

</details>

<details>
<summary><strong>Decibels and cable attenuation</strong></summary>

Power ratio in decibels:

$$
\Delta P_{\mathrm{dB}}=10\log_{10}\left(\frac{P_2}{P_1}\right)
$$

For twice the power:

$$
10\log_{10}(2)\approx3.01\text{ dB}
$$

Useful approximations:

- doubling power is approximately **+3 dB**;
- halving power is approximately **−3 dB**; and
- multiplying power by ten is **+10 dB**.

A simplified frequency-dependent copper-cable loss model is:

$$
\alpha\approx k_1\sqrt{f}+k_2f
$$

where the $\sqrt{f}$ term models conductor/skin-effect loss and the $f$ term models dielectric loss. The constants depend on the cable construction and materials.

</details>

<details>
<summary><strong>Nonlinear loads and harmonics</strong></summary>

A nonlinear load does not draw current in proportion to the applied sinusoidal voltage. The distorted current waveform can be represented as a fundamental component plus harmonic components:

$$
i(t)=I_1\sin(\omega t+\phi_1)+\sum_{n=2}^{\infty}I_n\sin(n\omega t+\phi_n)
$$

Study cues:

- electronic power supplies and rectifier-input loads commonly produce current harmonics;
- harmonics increase current distortion and can add heating in conductors and transformers;
- triplen harmonics in four-wire wye systems can add in the neutral rather than cancel; and
- distinguish **displacement power factor** from **true power factor** when waveform distortion is present.

> [!NOTE]
> This is general trade theory. For an NEC question involving neutral loading, conductor sizing, or nonlinear loads, locate and apply the exact rule from the exam edition of the code.

</details>

</details>

---

<a id="practice"></a>

<details>
<summary><strong>07 / Ampere-Turn Practice</strong></summary>

Use:

$$
\mathcal{F}=NI
$$

Try each problem before opening its answer.

<details>
<summary><strong>Problem 1 — Three turns carrying 12 A</strong></summary>

A coil has 3 turns and carries 12 A. What is its magnetomotive force?

- A. 9 ampere-turns
- B. 15 ampere-turns
- C. 36 ampere-turns
- D. 48 ampere-turns

<details>
<summary><strong>Show answer</strong></summary>

**C. 36 ampere-turns**

$$
\mathcal{F}=NI=3(12)=36\text{ ampere-turns}
$$

</details>

</details>

<details>
<summary><strong>Problem 2 — Five turns carrying 4 A</strong></summary>

A coil has 5 turns and carries 4 A. What is the total ampere-turn value?

- A. 9 ampere-turns
- B. 20 ampere-turns
- C. 25 ampere-turns
- D. 40 ampere-turns

<details>
<summary><strong>Show answer</strong></summary>

**B. 20 ampere-turns**

$$
\mathcal{F}=NI=5(4)=20\text{ ampere-turns}
$$

</details>

</details>

<details>
<summary><strong>Problem 3 — Solve for current</strong></summary>

A six-turn coil produces 18 ampere-turns. What current flows in the coil?

- A. 2 A
- B. 3 A
- C. 6 A
- D. 12 A

<details>
<summary><strong>Show answer</strong></summary>

**B. 3 A**

$$
I=\frac{\mathcal{F}}{N}=\frac{18}{6}=3\text{ A}
$$

</details>

</details>

<details>
<summary><strong>Problem 4 — Solve for turns</strong></summary>

A coil produces 32 ampere-turns while carrying 8 A. How many turns does it have?

- A. 2 turns
- B. 4 turns
- C. 6 turns
- D. 8 turns

<details>
<summary><strong>Show answer</strong></summary>

**B. 4 turns**

$$
N=\frac{\mathcal{F}}{I}=\frac{32}{8}=4\text{ turns}
$$

</details>

</details>

<details>
<summary><strong>Problem 5 — Seven turns carrying 9 A</strong></summary>

A coil has 7 turns and carries 9 A. What magnetomotive force does it produce?

- A. 16 ampere-turns
- B. 56 ampere-turns
- C. 63 ampere-turns
- D. 72 ampere-turns

<details>
<summary><strong>Show answer</strong></summary>

**C. 63 ampere-turns**

$$
\mathcal{F}=NI=7(9)=63\text{ ampere-turns}
$$

</details>

</details>

<details>
<summary><strong>Problem 6 — Required current</strong></summary>

A nine-turn coil must produce 45 ampere-turns. What current is required?

- A. 3 A
- B. 4 A
- C. 5 A
- D. 6 A

<details>
<summary><strong>Show answer</strong></summary>

**C. 5 A**

$$
I=\frac{\mathcal{F}}{N}=\frac{45}{9}=5\text{ A}
$$

</details>

</details>

<details>
<summary><strong>Problem 7 — Power over a 48 V circuit</strong></summary>

A 48 V circuit supplies 2.5 A. What power is delivered?

- A. 19.2 W
- B. 50.5 W
- C. 120 W
- D. 192 W

<details>
<summary><strong>Show answer</strong></summary>

**C. 120 W**

$$
P=VI=48(2.5)=120\text{ W}
$$

</details>

</details>

</details>

---

<a id="resources"></a>

<details>
<summary><strong>08 / Official References and Study Resources</strong></summary>

<details open>
<summary><strong>Primary exam and legal references</strong></summary>

| Reference | Use |
|---|---|
| [Washington L&I — Telecommunications Administrator](https://lni.wa.gov/licensing-permits/electrical/electrical-licensing-exams-education/telecommunications-administrator) | Certification, application, assignment, and renewal information |
| [PSI Candidate Information Bulletin](https://lni.wa.gov/licensing-permits/_docs/display_bulletin.pdf) | Exam content, timing, permitted materials, and rules |
| [Washington L&I — Electrical Laws, Rules, and Policies](https://www.lni.wa.gov/licensing-permits/electrical/laws-rules-policies) | Current official RCW and WAC links |
| [Exam-friendly RCW 19.28](https://lni.wa.gov/forms-publications/F500-143-000.pdf) | Washington electrical law reference |
| [Exam-friendly WAC 296-46B](https://lni.wa.gov/forms-publications/F500-142-000.pdf) | Washington electrical rule reference |
| [NFPA 70 Code Development and Free Access](https://www.nfpa.org/codes-and-standards/nfpa-70-standard-development/70) | Official NEC information and read-only access |
| [NFPA Free Access List](https://www.nfpa.org/for-professionals/codes-and-standards/list-of-codes-and-standards/free-access) | Official free online codes and standards |

> [!CAUTION]
> Use official NFPA access or a lawfully obtained code book. Unofficial PDF mirrors may be outdated, incomplete, altered, or unauthorized.

</details>

<details>
<summary><strong>Chapter 8 communications study links</strong></summary>

- [EC&M — Article 800 Code Basics](https://www.ecmweb.com/national-electrical-code/code-basics/article/20890128/article-800-communications-circuits)
- [EC&M — Communications Systems Grounding Rules](https://www.ecmag.com/magazine/articles/article-detail/codes-standards-communications-systems-grounding-rules-article-800-provides-specific-requirements)
- [Mike Holt — Communications Systems study material](https://www.mikeholt.com/newsletters.php?action=display&letterID=2536)
- [MyNFPA70 — Chapter 8 index](https://mynfpa70.com/mynfpa70welcome/chapter8/)
- [Nassau National Cable — Article 800 overview](https://nassaunationalcable.com/blogs/blog/explaining-nec-article-800-on-communication-circuits)

Treat third-party material as commentary. Resolve conflicts by using the adopted code and official Washington rules.

</details>

<details>
<summary><strong>Structured cabling and industry references</strong></summary>

- [BICSI](https://www.bicsi.org)
- [EC&M NEC Code Basics library](https://www.ecmweb.com/national-electrical-code/code-basics)
- [NFPA — Understanding NFPA 70](https://www.nfpa.org/education-and-research/electrical/understanding-nfpa-70-national-electrical-code)
- [Fresno State Telecommunications Infrastructure Design Standards](https://technology.fresnostate.edu/documents/1-FSU%20TIDS%20ver%2011%205-10-17.pdf)

</details>

<details>
<summary><strong>Practice tests and exam preparation</strong></summary>

- [ContractorTests — Washington 09 Telecommunications Administrator practice test](https://www.contractortests.com/shop/washington-09-telecommunications-administrator-practice-test/)
- [DakotaPrep — Washington electrical administrator exam preparation](https://www.dakotaprep.com/states/pass-your-administrator-journeyman-and-master-electrical-exam-in-washington)
- [Electrician Exam Practice Tests — Ohm's law](https://www.electricianexampracticetests.com/electrical-theory/ohms-law-test-questions/)
- [Thompson Learning — Free electrician practice test](https://thompsonlearningco.com/free-electrician-exam-practice-test-2020-nec/)
- [YouTube — Washington 09 Telecommunications exam](https://www.youtube.com/watch?v=cucCX4Xes58)
- [YouTube — Washington Electrical Administrator exam strategy](https://www.youtube.com/watch?v=Fp7qjXnnml4)

</details>

</details>

---

<a id="anki"></a>

<details>
<summary><strong>09 / Anki Study Workflow</strong></summary>

<details open>
<summary><strong>Applications</strong></summary>

- [Anki Desktop](https://apps.ankiweb.net/)
- [AnkiWeb](https://www.ankiweb.net)
- [AnkiDroid for Android](https://play.google.com/store/apps/details?id=com.ichi2.anki)
- [Anki tutorial](https://www.youtube.com/watch?v=0P3dkF9oCZw)

</details>

<details>
<summary><strong>Suggested FSRS settings</strong></summary>

| Setting | Suggested value | Reason |
|---|---|---|
| Scheduler | Modern/V3 scheduler | Required by current FSRS workflows |
| FSRS | Enabled | Adapts review intervals to recall history |
| Desired retention | 90% to 95% | Higher retention increases workload |
| Learning steps | `25m` | One intraday learning step |
| Relearning steps | `10m` | Short lapse-recovery step |
| Graduating interval | `3d` | Initial post-learning interval |
| Easy interval | `4d` | Modest skip-ahead interval |
| New cards per day | Match your sustainable pace | Avoid creating an unmanageable review backlog |
| Maximum reviews per day | High enough to clear due cards | Prevent artificial review delays |

Button discipline:

- press <kbd>Again</kbd> when the answer was wrong;
- use <kbd>Good</kbd> as the default correct response;
- use <kbd>Hard</kbd> sparingly for correct but difficult recall; and
- use <kbd>Easy</kbd> only for immediate, effortless recall.

</details>

<details>
<summary><strong>Card design for a lookup examination</strong></summary>

Prefer cards that train navigation rather than cards that merely repeat summaries.

| Front | Back |
|---|---|
| `Ferrous enclosure + induced current` | `Article 300 general requirements -> induced currents -> 300.20` |
| `Class 2 or Class 3 power-limited circuit` | `Chapter 7 -> Article 725 -> locate exact rule` |
| `Network-powered broadband` | `Chapter 8 -> Article 830` |
| `Coaxial CATV distribution` | `Chapter 8 -> Article 820` |
| `Current harmonics from nonlinear load` | `Theory concept first; then locate any applicable NEC neutral/conductor rule` |

> [!TIP]
> A strong lookup card includes the **question cue**, the **article path**, and the **final section or table**. Keep the quoted answer short enough that you still practice using the book.

</details>

</details>

---

<details>
<summary><strong>10 / One-Page Cram Sheet</strong></summary>

```text
EXAM
NEC + Theory: 30 questions / 2 hours / 70% to pass
WA Codes:      17 questions / 1 hour  / 70% to pass

PRIORITY
RCW/WAC 17 | NEC 800–830 8 | Theory 6 | NEC 705–780 4
NEC 100–110 3 | NEC 250 3 | NEC 300–398 3 | NEC 90 2 | NEC 500–516 1

CHAPTER 3 — CABFER
C 300–315 Core | A 320–340 Cables | B 342–362 Conduit/Tubing
F 366–371 Bus/Gutters | E 372–384 Floor/Wireways/Extensions
R 386–398 Raceways/Trays/Open Wiring

CHAPTER 7 — ESPOWER
700–710 Emergency/Standby/Special Power
722–728 Power-Limited/Fault-Managed/Fire-Resistive Cable
750 Energy Management | 760 Fire Alarm | 770 Optical Fiber

CHAPTER 8 — ANTENNA
800 General | 805 Comms Circuits | 810 Antennas
820 CATV | 830 Network-Powered | 840 Premises-Powered

THEORY
V = IR                    P = VI = I²R = V²/R
Vs/Vp = Ns/Np             Is/Ip = Np/Ns
Ampere-turns = NI         Vrms = 0.707 Vpeak
Xc = 1/(2πfC)             XL = 2πfL
P3φ = √3 × V × I × PF     dB = 10 log10(P2/P1)

PHASE
Inductive: current lags voltage
Capacitive: current leads voltage
Nonlinear: distorted current and harmonics; triplen harmonics can add in the neutral
```

</details>

---

<details>
<summary><strong>11 / Scope, Accuracy, and Maintenance</strong></summary>

This repository is an independent study aid. It is not affiliated with NFPA, PSI, or Washington State Labor & Industries, and it does not replace the NEC, RCW, WAC, the PSI Candidate Information Bulletin, or professional instruction.

- The Chapter 3, Chapter 7, and Chapter 8 maps follow the attached 2023 NEC-oriented study material.
- The official examination blueprint section follows the linked October 30, 2025 PSI bulletin, which identifies the 2020 NEC.
- Verify code quotations in the exact edition authorized for your examination.
- Verify Washington laws and rules against current official publications.
- Treat question counts as approximate, as stated by PSI.
- Treat formulas and worked examples as general trade theory.
- Report stale links or factual errors through the repository's issue tracker.

**Last reviewed:** July 14, 2026  
**Bulletin checked:** PSI Candidate Information Bulletin dated October 30, 2025

</details>

<div align="center">
  <sub>Built for fast lookup, deliberate practice, and calm exam-day navigation.</sub>
</div>
