export interface FieldInit { name: string; dataType: string; description: string; semanticTypeValue?: string }
export class Field {
  readonly name: string; readonly dataType: string; readonly description: string; readonly semanticTypeValue?: string;
  constructor(init: FieldInit) { this.name = init.name; this.dataType = init.dataType; this.description = init.description; this.semanticTypeValue = init.semanticTypeValue; }
  semanticType(value: string): Field { return new Field({ ...this, semanticTypeValue: value }); }
}

export interface FileObjectInit { id: string; name: string; contentUrl: string; encodingFormat: string }
export class FileObject { readonly id: string; readonly name: string; readonly contentUrl: string; readonly encodingFormat: string;
  constructor(init: FileObjectInit) { Object.assign(this, init); }
}

export interface RecordSetInit { id: string; name: string; fields?: Field[] }
export class RecordSet { readonly id: string; readonly name: string; readonly fields: Field[];
  constructor(init: RecordSetInit) { this.id = init.id; this.name = init.name; this.fields = init.fields ?? []; }
}

export interface CroissantDatasetInit { id: string; name: string; description: string; license: string; creators: string[]; files: FileObject[]; recordSets: RecordSet[]; keywords: string[] }
export class CroissantDataset {
  readonly id: string; readonly name: string; readonly description: string; readonly license: string; readonly creators: string[]; readonly files: FileObject[]; readonly recordSets: RecordSet[]; readonly keywords: string[];
  constructor(init: CroissantDatasetInit) { Object.assign(this, init); }
  toJsonLd(): Record<string, unknown> {
    return { "@context": { "@vocab": "https://schema.org/", cr: "http://mlcommons.org/croissant/", dcat: "http://www.w3.org/ns/dcat#", odrl: "http://www.w3.org/ns/odrl/2/" }, "@type": "cr:Dataset", "@id": this.id, name: this.name, description: this.description, license: this.license, creator: this.creators.map((name) => ({ "@type": "Person", name })), keywords: this.keywords,
      distribution: this.files.map((file) => ({ "@type": "cr:FileObject", "@id": file.id, name: file.name, contentUrl: file.contentUrl, encodingFormat: file.encodingFormat })),
      recordSet: this.recordSets.map((recordSet) => ({ "@type": "cr:RecordSet", "@id": recordSet.id, name: recordSet.name, field: recordSet.fields.map((field) => ({ "@type": "cr:Field", name: field.name, dataType: field.dataType, description: field.description, sameAs: field.semanticTypeValue ?? null })) })),
    };
  }
}
