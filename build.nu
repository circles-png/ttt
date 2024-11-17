cargo build -r
avr-objcopy -I elf32-avr -O ihex -j .eeprom --set-section-flags=.eeprom=alloc,load --no-change-warnings --change-section-lma .eeprom=0 target/avr-atmega328p/release/ttt.elf out/ttt.eep
avr-objcopy -I elf32-avr -O ihex -R .eeprom target/avr-atmega328p/release/ttt.elf out/ttt.hex
avr-size -C target/avr-atmega328p/release/ttt.elf
avrdude -v -p atmega328pb -P /dev/tty.usbmodem101 -b19200 -c stk500v1 -U out/ttt.hex -C /Applications/Arduino.app/Contents/Java/hardware/tools/avr/etc/avrdude.conf
