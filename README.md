# **d**omenic **v**irtual **m**achine - dvm

This custom virtual machine has been created in the course [194.160 Abstract Machines](https://tiss.tuwien.ac.at/course/courseDetails.xhtml?dswid=2087&dsrid=583&locale=en&courseNr=194160) at the TU Wien in 2026S.

## Opcodes

TODO create table

## Perf

Building binary in with profile `profiling` and then gathering some data with the help of [samply](https://github.com/mstange/samply).

```sh
$ cargo b --profile profiling --features profiling 
$ samply record ./target/profiling/dvm run SOME_FILE
```
