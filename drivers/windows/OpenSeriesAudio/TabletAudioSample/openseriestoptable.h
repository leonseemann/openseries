/*++

Copyright (c) Microsoft Corporation All Rights Reserved
Modifications for OpenSeries: copyright the OpenSeries contributors.

Licensed under the Microsoft Public License (MS-PL), see ..\LICENSE-MS-PL.

Module Name:

    openseriestoptable.h

Abstract:

    Topology tables of the four OpenSeries render sinks (Game, Chat, Media, Aux).
    Derived from SysVAD's speakertoptable.h: same pins, connections and automation table, but
    the line-out bridge pin carries a per-endpoint Name GUID so Windows shows the names
    registered under MediaCategories in OpenSeriesAudio.inx ("OpenSeries Game" ...).
    docs/architecture/windows-virtual-driver.md, section 1.

--*/

#ifndef _OPENSERIES_TOPTABLE_H_
#define _OPENSERIES_TOPTABLE_H_

#include "speakertoptable.h"

// Endpoint names; keep in sync with the *NameGUID strings in OpenSeriesAudio.inx.
DEFINE_GUID(OPENSERIES_GAME_NAME,
0x088779c2, 0xa08d, 0x4a2e, 0x96, 0x71, 0x5e, 0x2b, 0xaf, 0xe0, 0x32, 0xd0);
DEFINE_GUID(OPENSERIES_CHAT_NAME,
0x54ce2ba9, 0xeb18, 0x4e21, 0x97, 0x85, 0xa5, 0x71, 0xc3, 0x6b, 0x07, 0x21);
DEFINE_GUID(OPENSERIES_MEDIA_NAME,
0xf8b4e0ec, 0x12f6, 0x45e5, 0x87, 0x15, 0x2d, 0x58, 0xdf, 0xd0, 0x72, 0x80);
DEFINE_GUID(OPENSERIES_AUX_NAME,
0x8533faea, 0x675f, 0x4bc5, 0x9d, 0xd7, 0xad, 0x12, 0x9d, 0x1f, 0x1a, 0xa2);

// Pins and filter descriptor of one sink's topology filter. Identical to the SysVAD speaker
// topology except for the Name of the KSPIN_TOPO_LINEOUT_DEST pin.
#define OPENSERIES_SINK_TOPOLOGY(Channel, NameGuid)                                         \
static                                                                                       \
PCPIN_DESCRIPTOR Channel##TopoMiniportPins[] =                                               \
{                                                                                            \
  /* KSPIN_TOPO_WAVEOUT_SOURCE */                                                            \
  {                                                                                          \
    0, 0, 0, NULL,                                                                           \
    {                                                                                        \
      0, NULL, 0, NULL,                                                                      \
      SIZEOF_ARRAY(SpeakerTopoPinDataRangePointersBridge),                                   \
      SpeakerTopoPinDataRangePointersBridge,                                                 \
      KSPIN_DATAFLOW_IN, KSPIN_COMMUNICATION_NONE,                                           \
      &KSCATEGORY_AUDIO, NULL, 0                                                             \
    }                                                                                        \
  },                                                                                         \
  /* KSPIN_TOPO_LINEOUT_DEST */                                                              \
  {                                                                                          \
    0, 0, 0, NULL,                                                                           \
    {                                                                                        \
      0, NULL, 0, NULL,                                                                      \
      SIZEOF_ARRAY(SpeakerTopoPinDataRangePointersBridge),                                   \
      SpeakerTopoPinDataRangePointersBridge,                                                 \
      KSPIN_DATAFLOW_OUT, KSPIN_COMMUNICATION_NONE,                                          \
      &KSNODETYPE_SPEAKER, &NameGuid, 0                                                      \
    }                                                                                        \
  }                                                                                          \
};                                                                                           \
                                                                                             \
static                                                                                       \
PCFILTER_DESCRIPTOR Channel##TopoMiniportFilterDescriptor =                                  \
{                                                                                            \
  0,                                                                                         \
  &AutomationSpeakerTopoFilter,                                                              \
  sizeof(PCPIN_DESCRIPTOR),                                                                  \
  SIZEOF_ARRAY(Channel##TopoMiniportPins),                                                   \
  Channel##TopoMiniportPins,                                                                 \
  sizeof(PCNODE_DESCRIPTOR),                                                                 \
  0,                                                                                         \
  NULL,                                                                                      \
  SIZEOF_ARRAY(SpeakerTopoMiniportConnections),                                              \
  SpeakerTopoMiniportConnections,                                                            \
  0,                                                                                         \
  NULL                                                                                       \
};

OPENSERIES_SINK_TOPOLOGY(Game, OPENSERIES_GAME_NAME)
OPENSERIES_SINK_TOPOLOGY(Chat, OPENSERIES_CHAT_NAME)
OPENSERIES_SINK_TOPOLOGY(Media, OPENSERIES_MEDIA_NAME)
OPENSERIES_SINK_TOPOLOGY(Aux, OPENSERIES_AUX_NAME)

#endif // _OPENSERIES_TOPTABLE_H_
