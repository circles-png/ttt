cargo build -r
avr-objcopy -I elf32-avr -O ihex -R .eeprom target/avr-atmega328p/release/ttt.elf out/ttt.hex
avrdude -v -p atmega328pb -P /dev/tty.usbmodem101 -c stk500v1 -Uflash:w:out/ttt.hex:i -C /Applications/Arduino.app/Contents/Java/hardware/tools/avr/etc/avrdude.conf -b 19200
