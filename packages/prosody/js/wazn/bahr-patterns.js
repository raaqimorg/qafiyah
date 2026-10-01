import { BahrName, BaseTaffelah, TaffelahVariant } from '../enums.js';

import { BahrPattern } from './bahr-pattern.js';
import { Taffelah } from './taffelah.js';

const taweel = new BahrPattern({
  name: BahrName.taweel,
  maxLength: 24,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.faoolon,
      allowedVariants: [TaffelahVariant.faooloDamma, TaffelahVariant.faoolon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaeelon,
      allowedVariants: [TaffelahVariant.mafaaelon, TaffelahVariant.mafaaeelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faoolon,
      allowedVariants: [TaffelahVariant.faooloDamma, TaffelahVariant.faoolon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaeelon,
      allowedVariants: [TaffelahVariant.mafaaelon, TaffelahVariant.mafaaeelon],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.faoolon,
      allowedVariants: [TaffelahVariant.faooloDamma, TaffelahVariant.faoolon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaeelon,
      allowedVariants: [TaffelahVariant.mafaaelon, TaffelahVariant.mafaaeelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faoolon,
      allowedVariants: [TaffelahVariant.faooloDamma, TaffelahVariant.faoolon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaeelon,
      allowedVariants: [TaffelahVariant.mafaaelon, TaffelahVariant.mafaaeelon],
    }),
  ],
});

const basset = new BahrPattern({
  name: BahrName.basset,
  maxLength: 24,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faelon, TaffelahVariant.faaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faelon, TaffelahVariant.faael],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faelon, TaffelahVariant.faaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faelon, TaffelahVariant.faael],
    }),
  ],
});

const bassetPart = new BahrPattern({
  name: BahrName.bassetPart,
  maxLength: 17,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faoolon,
      allowedVariants: [TaffelahVariant.faoolon],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faoolon,
      allowedVariants: [TaffelahVariant.faoolon],
    }),
  ],
});

const kamel = new BahrPattern({
  name: BahrName.kamel,
  maxLength: 21,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.motafaaelon,
      allowedVariants: [TaffelahVariant.motafaaeloDamma, TaffelahVariant.motafaaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.motafaaelon,
      allowedVariants: [TaffelahVariant.motafaaeloDamma, TaffelahVariant.motafaaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.motafaaelon,
      allowedVariants: [
        TaffelahVariant.motSakinFa,
        TaffelahVariant.motSakinFaaelon,
        TaffelahVariant.motafaaelSakin,
        TaffelahVariant.motafaaeloDamma,
        TaffelahVariant.motafaaelon,
      ],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.motafaaelon,
      allowedVariants: [TaffelahVariant.motafaaeloDamma, TaffelahVariant.motafaaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.motafaaelon,
      allowedVariants: [TaffelahVariant.motafaaeloDamma, TaffelahVariant.motafaaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.motafaaelon,
      allowedVariants: [
        TaffelahVariant.motSakinFa,
        TaffelahVariant.motafaaeloDamma,
        TaffelahVariant.motafaaelon,
      ],
    }),
  ],
});

const kamelPart = new BahrPattern({
  name: BahrName.kamelPart,
  maxLength: 14,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.motafaaelon,
      allowedVariants: [TaffelahVariant.motafaaeloDamma, TaffelahVariant.motafaaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.motafaaelon,
      allowedVariants: [
        TaffelahVariant.motSakinFaaelaan,
        TaffelahVariant.motafaaelaan,
        TaffelahVariant.motSakinFaaelaaton,
        TaffelahVariant.motafaaelaaton,
        TaffelahVariant.motSakinFaaelon,
        TaffelahVariant.motafaaelSakin,
        TaffelahVariant.motafaaeloDamma,
        TaffelahVariant.motafaaelon,
      ],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.motafaaelon,
      allowedVariants: [TaffelahVariant.motafaaeloDamma, TaffelahVariant.motafaaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.motafaaelon,
      allowedVariants: [
        TaffelahVariant.motSakinFaaelaan,
        TaffelahVariant.motafaaelaan,
        TaffelahVariant.motSakinFaaelaaton,
        TaffelahVariant.motafaaelaaton,
        TaffelahVariant.motSakinFaaelon,
        TaffelahVariant.motafaaelSakin,
        TaffelahVariant.motafaaeloDamma,
        TaffelahVariant.motafaaelon,
      ],
    }),
  ],
});

const wafer = new BahrPattern({
  name: BahrName.wafer,
  maxLength: 19,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaelaton,
      allowedVariants: [TaffelahVariant.mafaaelSakinTon, TaffelahVariant.mafaaelaton],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaelaton,
      allowedVariants: [TaffelahVariant.mafaaelSakinTon, TaffelahVariant.mafaaelaton],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faoolon,
      allowedVariants: [TaffelahVariant.faoolon],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaelaton,
      allowedVariants: [TaffelahVariant.mafaaelSakinTon, TaffelahVariant.mafaaelaton],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaelaton,
      allowedVariants: [TaffelahVariant.mafaaelSakinTon, TaffelahVariant.mafaaelaton],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faoolon,
      allowedVariants: [TaffelahVariant.faoolon],
    }),
  ],
});

const waferPart = new BahrPattern({
  name: BahrName.waferPart,
  maxLength: 14,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaelaton,
      allowedVariants: [TaffelahVariant.mafaaelSakinTon, TaffelahVariant.mafaaelaton],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaelaton,
      allowedVariants: [TaffelahVariant.mafaaelaton],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaelaton,
      allowedVariants: [TaffelahVariant.mafaaelSakinTon, TaffelahVariant.mafaaelaton],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaelaton,
      allowedVariants: [TaffelahVariant.mafaaelSakinTon, TaffelahVariant.mafaaelaton],
    }),
  ],
});

const khafeef = new BahrPattern({
  name: BahrName.khafeef,
  maxLength: 21,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [TaffelahVariant.faelaton, TaffelahVariant.faaelaton],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [
        TaffelahVariant.faaelSakinTon,
        TaffelahVariant.faela,
        TaffelahVariant.faelaton,
        TaffelahVariant.faaelaton,
      ],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [TaffelahVariant.faelaton, TaffelahVariant.faaelaton],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [
        TaffelahVariant.faaelSakinTon,
        TaffelahVariant.faela,
        TaffelahVariant.faelaton,
        TaffelahVariant.faaelaton,
      ],
    }),
  ],
});

const khafeefPart = new BahrPattern({
  name: BahrName.khafeefPart,
  maxLength: 14,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [TaffelahVariant.faelaton, TaffelahVariant.faaelaton],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [TaffelahVariant.motafeelon, TaffelahVariant.mostafeelon],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [TaffelahVariant.faelaton, TaffelahVariant.faaelaton],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [TaffelahVariant.motafeelon, TaffelahVariant.mostafeelon],
    }),
  ],
});

const ramal = new BahrPattern({
  name: BahrName.ramal,
  maxLength: 21,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [TaffelahVariant.faelaton, TaffelahVariant.faaelaton],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [TaffelahVariant.faelaton, TaffelahVariant.faaelaton],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [
        TaffelahVariant.faelatSakin,
        TaffelahVariant.faaelat,
        TaffelahVariant.faela,
        TaffelahVariant.faaela,
        TaffelahVariant.faelaton,
        TaffelahVariant.faaelaton,
      ],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [TaffelahVariant.faelaton, TaffelahVariant.faaelaton],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [TaffelahVariant.faelaton, TaffelahVariant.faaelaton],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [
        TaffelahVariant.faelatSakin,
        TaffelahVariant.faaelat,
        TaffelahVariant.faela,
        TaffelahVariant.faaela,
        TaffelahVariant.faelaton,
        TaffelahVariant.faaelaton,
      ],
    }),
  ],
});

const ramalPart = new BahrPattern({
  name: BahrName.ramalPart,
  maxLength: 14,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [TaffelahVariant.faelaton, TaffelahVariant.faaelaton],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [
        TaffelahVariant.faelaton,
        TaffelahVariant.faaelaton,
        TaffelahVariant.faela,
        TaffelahVariant.faaela,
        TaffelahVariant.faelaton,
        TaffelahVariant.faaelaton,
      ],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [TaffelahVariant.faelaton, TaffelahVariant.faaelaton],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [
        TaffelahVariant.faelaton,
        TaffelahVariant.faaelaton,
        TaffelahVariant.faela,
        TaffelahVariant.faaela,
        TaffelahVariant.faelaton,
        TaffelahVariant.faaelaton,
      ],
    }),
  ],
});

const motokareb = new BahrPattern({
  name: BahrName.motokareb,
  maxLength: 20,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.faoolon,
      allowedVariants: [TaffelahVariant.faooloDamma, TaffelahVariant.faoolon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faoolon,
      allowedVariants: [TaffelahVariant.faooloDamma, TaffelahVariant.faoolon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faoolon,
      allowedVariants: [TaffelahVariant.faooloDamma, TaffelahVariant.faoolon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faoolon,
      allowedVariants: [
        TaffelahVariant.faoolSakin,
        TaffelahVariant.fae,
        TaffelahVariant.faoo,
        TaffelahVariant.faooloDamma,
        TaffelahVariant.faoolon,
      ],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.faoolon,
      allowedVariants: [TaffelahVariant.faooloDamma, TaffelahVariant.faoolon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faoolon,
      allowedVariants: [TaffelahVariant.faooloDamma, TaffelahVariant.faoolon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faoolon,
      allowedVariants: [TaffelahVariant.faooloDamma, TaffelahVariant.faoolon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faoolon,
      allowedVariants: [
        TaffelahVariant.faoolSakin,
        TaffelahVariant.fae,
        TaffelahVariant.faoo,
        TaffelahVariant.faooloDamma,
        TaffelahVariant.faoolon,
      ],
    }),
  ],
});

const monsareh = new BahrPattern({
  name: BahrName.monsareh,
  maxLength: 21,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.motaelon,
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafeoolaat,
      allowedVariants: [TaffelahVariant.faaelaan, TaffelahVariant.mafeoolaat],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.mostafeelSakin,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.motaelon,
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafeoolaat,
      allowedVariants: [TaffelahVariant.faaelaan, TaffelahVariant.mafeoolaat],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.mostafeelSakin,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
  ],
});

const madeed = new BahrPattern({
  name: BahrName.madeed,
  maxLength: 19,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [TaffelahVariant.faelaton, TaffelahVariant.faaelaton],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faelon, TaffelahVariant.faaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [
        TaffelahVariant.faael,
        TaffelahVariant.faelatSakin,
        TaffelahVariant.faaelat,
        TaffelahVariant.faela,
        TaffelahVariant.faaela,
        TaffelahVariant.faelaton,
        TaffelahVariant.faaelaton,
      ],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [TaffelahVariant.faelaton, TaffelahVariant.faaelaton],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faelon, TaffelahVariant.faaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [
        TaffelahVariant.faael,
        TaffelahVariant.faelatSakin,
        TaffelahVariant.faaelat,
        TaffelahVariant.faela,
        TaffelahVariant.faaela,
        TaffelahVariant.faelaton,
        TaffelahVariant.faaelaton,
      ],
    }),
  ],
});

const sareeh = new BahrPattern({
  name: BahrName.sareeh,
  maxLength: 19,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.motaelon,
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.motaelon,
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faael, TaffelahVariant.faelon, TaffelahVariant.faaelon],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.motaelon,
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.motaelon,
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faael, TaffelahVariant.faelon, TaffelahVariant.faaelon],
    }),
  ],
});

const rajaz = new BahrPattern({
  name: BahrName.rajaz,
  maxLength: 21,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.motaelon,
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.motaelon,
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.mostafeelSakin,
        TaffelahVariant.motaelon,
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.motaelon,
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.motaelon,
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.mostafeelSakin,
        TaffelahVariant.motaelon,
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
  ],
});

const rajazPart = new BahrPattern({
  name: BahrName.rajazPart,
  maxLength: 14,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.motaelon,
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.motaelon,
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.motaelon,
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.motaelon,
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
  ],
});

const hazaj = new BahrPattern({
  name: BahrName.hazaj,
  maxLength: 14,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaeelon,
      allowedVariants: [TaffelahVariant.mafaaeeloDamma, TaffelahVariant.mafaaeelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaeelon,
      allowedVariants: [TaffelahVariant.mafaaeeloDamma, TaffelahVariant.mafaaeelon],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaeelon,
      allowedVariants: [TaffelahVariant.mafaaeeloDamma, TaffelahVariant.mafaaeelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaeelon,
      allowedVariants: [TaffelahVariant.mafaaeeloDamma, TaffelahVariant.mafaaeelon],
    }),
  ],
});

const mohdath = new BahrPattern({
  name: BahrName.mohdath,
  maxLength: 20,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faael, TaffelahVariant.faelon, TaffelahVariant.faaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faael, TaffelahVariant.faelon, TaffelahVariant.faaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faael, TaffelahVariant.faelon, TaffelahVariant.faaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faael, TaffelahVariant.faelon, TaffelahVariant.faaelon],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faael, TaffelahVariant.faelon, TaffelahVariant.faaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faael, TaffelahVariant.faelon, TaffelahVariant.faaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faael, TaffelahVariant.faelon, TaffelahVariant.faaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faael, TaffelahVariant.faelon, TaffelahVariant.faaelon],
    }),
  ],
});

const modareeh = new BahrPattern({
  name: BahrName.modareeh,
  maxLength: 13,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaeelon,
      allowedVariants: [TaffelahVariant.mafaaeeloDamma],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [TaffelahVariant.faaelaton],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaeelon,
      allowedVariants: [TaffelahVariant.mafaaeeloDamma],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [TaffelahVariant.faaelaton],
    }),
  ],
});

const mojtath = new BahrPattern({
  name: BahrName.mojtath,
  maxLength: 14,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [
        TaffelahVariant.faaelSakinTon,
        TaffelahVariant.faelaton,
        TaffelahVariant.faaelaton,
      ],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.mostafeelon,
      allowedVariants: [
        TaffelahVariant.mostaelon,
        TaffelahVariant.motafeelon,
        TaffelahVariant.mostafeelon,
      ],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelaton,
      allowedVariants: [
        TaffelahVariant.faaelSakinTon,
        TaffelahVariant.faelaton,
        TaffelahVariant.faaelaton,
      ],
    }),
  ],
});

const moqtateb = new BahrPattern({
  name: BahrName.moqtateb,
  maxLength: 12,
  first: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaelaton,
      allowedVariants: [TaffelahVariant.mafaaelaton],
    }),
  ],
  second: [
    new Taffelah({
      baseTaffelah: BaseTaffelah.faaelon,
      allowedVariants: [TaffelahVariant.faaelon],
    }),
    new Taffelah({
      baseTaffelah: BaseTaffelah.mafaaelaton,
      allowedVariants: [TaffelahVariant.mafaaelaton],
    }),
  ],
});

export const allBahr = new Map([
  [BahrName.taweel, taweel],
  [BahrName.basset, basset],
  [BahrName.bassetPart, bassetPart],
  [BahrName.kamel, kamel],
  [BahrName.kamelPart, kamelPart],
  [BahrName.wafer, wafer],
  [BahrName.waferPart, waferPart],
  [BahrName.khafeef, khafeef],
  [BahrName.khafeefPart, khafeefPart],
  [BahrName.ramal, ramal],
  [BahrName.ramalPart, ramalPart],
  [BahrName.motokareb, motokareb],
  [BahrName.monsareh, monsareh],
  [BahrName.madeed, madeed],
  [BahrName.sareeh, sareeh],
  [BahrName.rajaz, rajaz],
  [BahrName.rajazPart, rajazPart],
  [BahrName.hazaj, hazaj],
  [BahrName.mohdath, mohdath],
  [BahrName.modareeh, modareeh],
  [BahrName.mojtath, mojtath],
  [BahrName.moqtateb, moqtateb],
]);
