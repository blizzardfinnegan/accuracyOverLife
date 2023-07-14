# WACP Communication

## Packet formatting

```
Preamble:               17 01 0c
Packet length:          XX XX XX XX
Port:                   01 19 (rendezvous) / 01 1A (WACP)
    Msg. Class ID:      XX XX XX XX 
    Msg. Size:          XX XX XX XX
    encrypt/comp:       00 
    object size:        XX XX XX XX [if 0x0, no object]
        obj. ClassID:   XX XX XX XX 
        obj. size:      2-byte or 4-byte, depending on ClassID
        obj. version:   XX XX
        bitmask:        00

        static Payload: 2 bytes to define staticly-sized variable's size, followed by statically sized values.
        OR
        Dyn. payload:   2 bytes set to zero, followed by (variable size, variable data) pairs.

        obj. CRC:       XX XX
    Msg CRC:            XX XX
packet CRC:             XX XX
```

4-byte object classes: 

- Class groups
    - `00 17 00 00`: Class session mask
        - exception: `0x00171000` is a 2-byte size parameter
    - `00 5c 00 00`: Collection mask
    - `00 00 12 00`: Extended size mask
- Individual classes:
    - `00 01 00 00`: CECGDPacer object
    - `00 1B 00 00`: CSpiroDStd object

CRC calculations documented farther down

## Rendezvous conversation
Host:   `0x17010c000000250119001d0b0100000012000000000b524e445a434f4e4e45435457f3e929`  [37 bytes long]

Device: `0x17010c0000001a0119001d0f0100000007000000000083b99238`                        [26 bytes long]

Host:   `0x17010c0000003b0119001d0102000000280000000021001d0000001b0065000012ffffffffffffffffffffffffffffffff00000000c01b499ed576`

Device: `0x17010c0000003b0119001d0103000000280000000021001d0000001b0065000012ffffffffffffffffffffffffffffffff00000000c01b265a85f8`

### Rendezvous breakdown

#### Packet 1

```
Preamble:               17 01 0c
Packet length:          00 00 00 25 (37 bytes long)
Port:                   01 19  (rendezvous) 
    Msg. Class ID:      00 1d 0b 01 
    Msg. Size:          00 00 00 12
    encrypt/comp:       00 
    object size:        00 00 000b
        Rendezvous obj: 52 4e 44 5a 43 4f 4e 4e 45 43 54
    Msg CRC:            57 f3
packet CRC:             e9 29
```

#### Packet 2

```
Preamble:               17 01 0c
Packet length:          00 00 00 1a (26 bytes long)
Port:                   01 19  (rendezvous) 
    Msg. Class ID:      00 1d 0f 01 
    Msg. Size:          00 00 00 07
    encrypt/comp:       00 
    object size:        00 00 00 00
    Msg CRC:            83 b9
packet CRC:             92 38
```

#### Packet 3

```
Preamble:               17 01 0c
Packet length:          00 00 00 3b (59 bytes long)
Port:                   01 19  (rendezvous) 
    Msg. Class ID:      00 1d 01 02 
    Msg. Size:          00 00 00 28
    encrypt/comp:       00 
    object size:        00 00 00 21
        obj. ClassID:   00 1d 00 00 
        obj. size:      00 1b
        obj. version:   00 65
        bitmask:        00
        Variable size:  00 12
        Client GUID:    ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
        DDS Version:    00 00
        Dyn. data size: 00 00
        obj. CRC:       c0 1b
    Msg CRC:            49 9e
packet CRC:             d5 76
```

#### Packet 4

```
Preamble:               17 01 0c
Packet length:          00 00 00 3b (59 bytes long)
Port:                   01 19  (rendezvous) 
    Msg. Class ID:      00 1d 01 03
    Msg. Size:          00 00 00 28
    encrypt/comp:       00 
    object size:        00 00 00 21
        obj. ClassID:   00 1d 00 00 
        obj. size:      00 1b
        obj. version:   00 65
        bitmask:        00
        Variable size:  00 12
                        e1 24 81 d1 cc b6 45 73 af 29 f1 63 42 53 85 cc
        Client GUID:    ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
                        00 21
        DDS Version:    00 00
        Dyn. data size: 00 00
        obj. CRC:       c0 1b
    Msg CRC:            26 5a
packet CRC:             85 f8
```

## WACP communication

### Serial request

```
Preamble:               17 01 0c
Packet length:          00 00 00 1a
Port:                   01 1A (WACP)
    Msg. Class ID:      00 18 0b 00  [FmDEVICE,GnRESPONSE,SpGET_DEVICEDESCRIPTION]
    Msg. Size:          00 00 00 07
    encrypt/comp:       00 
    object size:        00 00 00 00 
    Msg CRC:            71 e8
packet CRC:             1a 1f
```

### Serial Response

```
Preamble:               17 01 0c
Packet length:          00 00 00 95
Port:                   01 19 (rendezvous) / 01 1A (WACP)
    Msg. Class ID:      00 18 0f 00  [FmDEVICE,GnRESPONSE,SpPUT_DEVICEDESCRIPTION]
    Msg. Size:          00 00 00 82
    encrypt/comp:       00 
    object size:        00 00 00 7b 
        obj. ClassID:   00 18 00 00 [FmDEVICE,GnDATA,SpSTANDARD]
        obj. size:      00 00 00 72 [ClassID matches mask 00 5c 00 00]
        obj. version:   XX XX       [TBD]
        bitmask:        00
            Static size:00 6c       [108]
            datetime:   XX XX XX XX XX XX XX XX [Should be all zeros; no RTC on Disco]
            runtime:    XX XX XX XX             [Total on time for the unit since manufacture]
            model name: XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX [32x chars]
            S/N:        XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX [16x chars]
            GUID:       XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX [16x uint8]
            model #:    XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX XX [32x chars]
        obj. CRC:       XX XX
    Msg CRC:            XX XX
packet CRC:             XX XX
```

### Temp Request
\x17\x01\x0c\x00\x00\x00\x1a\x01\x1a\x00\x03\x0b\x00\x00\x00\x00\x07\x00\x00\x00\x00\x00\xe9\x32\x0e\xdf
```
Preamble:               17 01 0c
Packet length:          00 00 00 1a
Port:                   01 1A (WACP)
    Msg. Class ID:      00 03 0b 00 
    Msg. Size:          00 00 00 07
    encrypt/comp:       00 
    object size:        00 00 00 00
    Msg CRC:            e9 32
packet CRC:             0e df
```
### Temp Response

Failed temp:  17010c0000004e011900030f000000003b000000003400030001002e00cd000010000000000000000000030004000f0f0d000000130075001f000d00c80000064395ff7400049a2d6237885aac3b

Success temp: 17010c0000004e011900030f000000003b000000003400030001002e00cd000010000000000000000000010000000f0f0d000000130075001f000d00c80000064398ee520001bb0c039ae730852e
17010c0000001a0119
001d0f0100000007000000000083b99238

```
Preamble:                   17 01 0c
Packet length:              00 00 00 4e [78 bytes]
Port:                       01 19 	[rendezvous]
    Msg. Class ID:          00 03 0f 00 [FmTEMP (00 03),GnRESPONSE (0f),SpPUT_TEMP (00)]
    Msg. Size:              00 00 00 3b [59 bytes]
    encrypt/comp:           00          [uncompressed, unencrypted]
    object size:            00 00 00 34 [52 bytes]
        obj. ClassID:       00 03 00 01 [CTempDData {FmTEMP (00 03),GnDATA (00),SpDATA (01)}]
        obj. size:          00 2e       [46 bytes]
        obj. version:       00 cd       [v 205]
        bitmask:            00          [uncompressed, unencrypted]
        Static size:        00 10       [the next 16 bytes are static variables]
	static data:
            time:           00 00 00 00 00 00 00 00 [No RTC on Disco]
            status:         00 01       [Data complete]
            Ext. Status:    00 00       [No extended status]
            Source:         00 0f 	[Disco]
            mode of op.:    0f 		[Tympanic]
            Calc. method:   0d 		[Technique Compensation Calc]
	dynamic data:
        inner obj. size:    00 00 00 13 [The next 19 bits are a dynamic object]
            inner obj. id:  00 75 00 1f [CNumDFloat {FmNUMERIC (00 75),GnDATA (00),SpFLOAT (1f)}]
            obj size:       00 0d       [14 bytes]
            obj. vers:      00 c8       [v 200]
            bit mask:       00          [uncompressed, unencrypted]
            static size     00 06       [the next 6 bytes are a static variable]
                Temp (K):   43 98 ee 52 [305.861877K]
                Status:     00 01 	[Bitmask; Valid measurement]
            inner obj crc:  bb 0c 
        obj. CRC:           03 9a
    Msg CRC:                e7 30 
packet CRC:                 85 2e
```

## CRC Calculations

CRC is calculated using pregenerated table, plus the following function:

```
static const uint16 CRCTable[ 256 ] = {
    0x0000, 0x1189, 0x2312, 0x329b, 0x4624, 0x57ad, 0x6536, 0x74bf,
    0x8c48, 0x9dc1, 0xaf5a, 0xbed3, 0xca6c, 0xdbe5, 0xe97e, 0xf8f7,
    0x1081, 0x0108, 0x3393, 0x221a, 0x56a5, 0x472c, 0x75b7, 0x643e,
    0x9cc9, 0x8d40, 0xbfdb, 0xae52, 0xdaed, 0xcb64, 0xf9ff, 0xe876,
    0x2102, 0x308b, 0x0210, 0x1399, 0x6726, 0x76af, 0x4434, 0x55bd,
    0xad4a, 0xbcc3, 0x8e58, 0x9fd1, 0xeb6e, 0xfae7, 0xc87c, 0xd9f5,
    0x3183, 0x200a, 0x1291, 0x0318, 0x77a7, 0x662e, 0x54b5, 0x453c,
    0xbdcb, 0xac42, 0x9ed9, 0x8f50, 0xfbef, 0xea66, 0xd8fd, 0xc974,
    0x4204, 0x538d, 0x6116, 0x709f, 0x0420, 0x15a9, 0x2732, 0x36bb,
    0xce4c, 0xdfc5, 0xed5e, 0xfcd7, 0x8868, 0x99e1, 0xab7a, 0xbaf3,
    0x5285, 0x430c, 0x7197, 0x601e, 0x14a1, 0x0528, 0x37b3, 0x263a,
    0xdecd, 0xcf44, 0xfddf, 0xec56, 0x98e9, 0x8960, 0xbbfb, 0xaa72,
    0x6306, 0x728f, 0x4014, 0x519d, 0x2522, 0x34ab, 0x0630, 0x17b9,
    0xef4e, 0xfec7, 0xcc5c, 0xddd5, 0xa96a, 0xb8e3, 0x8a78, 0x9bf1,
    0x7387, 0x620e, 0x5095, 0x411c, 0x35a3, 0x242a, 0x16b1, 0x738,
    0xffcf, 0xee46, 0xdcdd, 0xcd54, 0xb9eb, 0xa862, 0x9af9, 0x8b70,
    0x8408, 0x9581, 0xa71a, 0xb693, 0xc22c, 0xd3a5, 0xe13e, 0xf0b7,
    0x0840, 0x19c9, 0x2b52, 0x3adb, 0x4e64, 0x5fed, 0x6d76, 0x7cff,
    0x9489, 0x8500, 0xb79b, 0xa612, 0xd2ad, 0xc324, 0xf1bf, 0xe036,
    0x18c1, 0x0948, 0x3bd3, 0x2a5a, 0x5ee5, 0x4f6c, 0x7df7, 0x6c7e,
    0xa50a, 0xb483, 0x8618, 0x9791, 0xe32e, 0xf2a7, 0xc03c, 0xd1b5,
    0x2942, 0x38cb, 0x0a50, 0x1bd9, 0x6f66, 0x7eef, 0x4c74, 0x5dfd,
    0xb58b, 0xa402, 0x9699, 0x8710, 0xf3af, 0xe226, 0xd0bd, 0xc134,
    0x39c3, 0x284a, 0x1ad1, 0x0b58, 0x7fe7, 0x6e6e, 0x5cf5, 0x4d7c,
    0xc60c, 0xd785, 0xe51e, 0xf497, 0x8028, 0x91a1, 0xa33a, 0xb2b3,
    0x4a44, 0x5bcd, 0x6956, 0x78df, 0x0c60, 0x1de9, 0x2f72, 0x3efb,
    0xd68d, 0xc704, 0xf59f, 0xe416, 0x90a9, 0x8120, 0xb3bb, 0xa232,
    0x5ac5, 0x4b4c, 0x79d7, 0x685e, 0x1ce1, 0x0d68, 0x3ff3, 0x2e7a,
    0xe70e, 0xf687, 0xc41c, 0xd595, 0xa12a, 0xb0a3, 0x8238, 0x93b1,
    0x6b46, 0x7acf, 0x4854, 0x59dd, 0x2d62, 0x3ceb, 0x0e70, 0x1ff9,
    0xf78f, 0xe606, 0xd49d, 0xc514, 0xb1ab, 0xa022, 0x92b9, 0x8330,
    0x7bc7, 0x6a4e, 0x58d5, 0x495c, 0x3de3, 0x2c6a, 0x1ef1, 0xf78
};

CRC=0xFFFF
for byte in buffer
    CRC = (CRC >> 8) XOR CRCTable[byte XOR (CRC AND 0x00FF)]

return CRC
```

---

# Code breakdown

## Message to send
- Message
    - Family:  FmTEMP
    - Genus:   GnREQUEST
    - Species: SpGET_TEMP
- Object: CSTPPSelector
    - Family:  FmTEMP
    - Genus:   GnPARAMETER
    - Species: SpSELECTOR
    - No members

## Message hopefully recieved
- Message
    - Family:    FmTEMP
    - Genus:     GnSTATUS
    - Species:   SpREPORT_TEMP
- Object: CTempDData
    - Family:    FmTEMP
    - Genus:     GnDATA
    - Species:   SpDATA
    - STime:     0
    - Status:    DATA_COMPLETE (hopefully)
    - ExtStatus: 0
    - Source:    ????
    - Mode:      ????
    - Method:    IR
    - Temp:      Temp (float) in kelvin, followed by a 16-bit unsigned int containing status bits

