import { ShatrHalf } from '../enums.js';
import { BahrSearch } from '../wazn/bahr-search.js';

export const analyzeBayt = (content) => {
  const search = new BahrSearch({ shatrsOfPoem: content.split('*') });
  search.setResultOfShatr();
  return {
    binary: search.numKetaba.slice(0, 2),
    arowdi: search.arowdiKetaba.slice(0, 2),
    bahrs: search.result.map((bahr) => ({
      name: bahr.whichBahr.label,
      first: bahr.taffelahLabels(ShatrHalf.first),
      second: bahr.taffelahLabels(ShatrHalf.second),
    })),
    rhyme: Object.fromEntries(
      [...search.result[0].rhymeResult].map(([term, value]) => [term.label, value])
    ),
  };
};
