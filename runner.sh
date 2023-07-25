#!/bin/bash

set -e

# flash
elf2uf2-rs -d "$1"

# debug uart
/Users/brandon/Desktop/defmt/target/debug/defmt-print -e "$1" serial --path /dev/tty.usbserial-A50285BI
