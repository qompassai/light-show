# Introduction

Light Show is a puzzle game about real Outside Plant (OSP) fiber-optic
engineering — the people who lay, splice, and troubleshoot the glass that
carries the internet to your house. It is written in Rust with the Bevy
game engine, runs on desktop and Android (Google Play and F-Droid), and
wraps the physics in a squad of anime-styled AI companions, one per access
technology, who react to every splice you make.

## The game in one sentence

Route light from the OLT (Point A) to the customer's ONT (Point B), and
land your received power inside the target window — while outages strike
and the clock runs.

## In plain terms: what you actually do

Think of it like connecting the dots, but every line you draw is a real
fiber-optic component with a real cost. Light leaves the transmitter at
some power level — say +3 dBm (roughly 2 milliwatts). Every piece of
glass, every splice, every connector, every splitter *eats* a little of
that light, measured in decibels (dB). Your job is to chain components
from A to B so that the light arriving at the far end falls inside the
level's target receive window — typically −27 dBm to −8 dBm for GPON
optics. Too much loss and the receiver can't see the signal; too little
loss and you overload ("blind") it.

You place components by **dragging from one node to another**. Where
several component types could fill the same gap, the game offers you a row
of *pills* — tap one to pick that variant explicitly. A live ledger at the
top of the board shows the running total: loss in dB, received power in
dBm, whether you're in window, and how many favor points you've earned.
The whole time, a companion sprite watches from the bottom of the screen
and reacts to your splices, outages, and results in her own voice.

Then the storm hits. Some levels script an outage — a backhoe severs a
buried span, a connector gets mated dirty, water creeps into a splice
closure. An alarm banner drops with a countdown, your companion flips to
alarmed, and you reroute or repair under time pressure. Fix the link budget
before the timer dies and it's a win; let it expire and the level ends as
a loss.

## Why the numbers are real

The simulation core (`osp_sim`, one of the two workspace crates) models
real-world loss figures: a fusion splice costs 0.075 dB while a mechanical
splice costs 0.4 dB; a UPC connector costs 0.35 dB versus 0.30 dB for APC;
a 1:32 PON splitter costs 17.7 dB; fiber attenuates 0.35/0.28/0.21 dB per
km at 1310/1490/1550 nm. These are the same numbers a real OSP tech reads
on an OTDR printout — and the game's ledger UI is deliberately styled like
one, so players learn to read a link budget the way techs do.

## What's in this book

The **User Guide** covers installing the game, the exact controls the
input code handles, the settings (there is no settings screen — see that
chapter for what this means), and the full menu → playing → results
workflow. The **Code** section documents the architecture and walks each
crate and module, grounded in the source as it exists today.

A note on honesty: every control, setting, and mechanic in the User Guide
was verified against the game's source code. Anything the code does not
actually implement is called out as unverifiable rather than documented as
if it existed.
