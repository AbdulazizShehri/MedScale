"""Generate MedScale's synthetic PII gold set (deterministic, no randomness).

Every name, date, place, number and address below is invented for MedScale
and refers to no real person. Offsets are Unicode scalar values, computed
from the assembled text, so the gold spans are exact by construction.

Usage: generate_pii_gold.py OUT_JSON [LANG]
  LANG: en (default), nl, fr, de, es, it or pt. Each language has its own
  natively phrased templates, names and date format; labels and the label map
  are shared.
"""

from __future__ import annotations

import json
import sys

# (first, last) pairs: invented; mixed scripts so offsets are tested beyond ASCII.
NAMES = [
    ("Lena", "Marsh"), ("Omar", "Haddad"), ("Priya", "Raman"), ("Jonas", "Weber"),
    ("Chloé", "Durand"), ("Mateo", "Ruiz"), ("Aiko", "Tanaka"), ("Samuel", "Okafor"),
    ("Ingrid", "Holm"), ("Yusuf", "Demir"), ("Clara", "Novak"), ("Ravi", "Menon"),
    ("Sofia", "Lindqvist"), ("Daniel", "Byrne"), ("Amara", "Nwosu"), ("Felix", "Brandt"),
    ("Noor", "Saleh"), ("Hugo", "Lemaire"), ("Maja", "Kowalska"), ("Tomás", "Ferreira"),
]
DATES = [
    "12 March 1980", "3 July 1975", "21 January 1992", "8 October 1968", "30 May 2001",
    "14 February 1959", "2 September 1987", "27 November 1971", "5 April 1996", "19 June 1983",
]
CITIES = ["Bristol", "Lyon", "Utrecht", "Porto", "Graz", "Aarhus", "Leeds", "Ghent", "Turin", "Malmö"]
PHONES = ["+44 7700 900123", "(555) 010-4477", "+31 20 555 0193", "+351 21 555 0172", "555-0136"]
EMAILS = [
    "lena.marsh@example.org", "o.haddad@example.net", "praman@example.com",
    "jonas.weber@example.org", "c.durand@example.net",
]
MRNS = ["4471902", "0093315", "7720481", "3358846", "6014729"]
AGES = ["34", "47", "61", "29", "72"]

# Each template is a list of literal strings and (field, label) slots.
TEMPLATES = [
    ["Patient ", ("first", "NAME"), " ", ("last", "NAME"), ", born ", ("date", "DATE"),
     ", lives in ", ("city", "LOCATION"), "."],
    [("first", "NAME"), " ", ("last", "NAME"), " was seen on ", ("date", "DATE"),
     " for follow-up of type 2 diabetes."],
    ["Contact ", ("first", "NAME"), " at ", ("phone", "PHONE"), " or ", ("email", "EMAIL"), "."],
    ["Record MRN ", ("mrn", "ID"), " belongs to ", ("first", "NAME"), " ", ("last", "NAME"), "."],
    ["The ", ("age", "AGE"), "-year-old patient from ", ("city", "LOCATION"),
     " reported chest pain on ", ("date", "DATE"), "."],
    ["Dr. ", ("last", "NAME"), " referred ", ("first", "NAME"), " to the clinic in ",
     ("city", "LOCATION"), "."],
    ["Please call ", ("phone", "PHONE"), " to reschedule the visit of ", ("first", "NAME"),
     " ", ("last", "NAME"), "."],
    ["Discharge summary for ", ("first", "NAME"), " ", ("last", "NAME"), " (MRN ", ("mrn", "ID"),
     "), admitted ", ("date", "DATE"), "."],
]

LABEL_MAP = {
    "first_name": "NAME", "last_name": "NAME",
    "date": "DATE", "date_of_birth": "DATE", "date_time": "DATE",
    "city": "LOCATION", "state": "LOCATION", "country": "LOCATION", "county": "LOCATION",
    "street_address": "LOCATION", "postcode": "LOCATION",
    "phone_number": "PHONE", "fax_number": "PHONE",
    "email": "EMAIL",
    "medical_record_number": "ID", "health_plan_beneficiary_number": "ID", "ssn": "ID",
    "account_number": "ID", "unique_id": "ID", "customer_id": "ID",
    "age": "AGE",
    # The multilingual OpenMed PII models use the ai4privacy-style schema
    # (FIRSTNAME, DATEOFBIRTH, PHONE, ...). It has no medical record number
    # label; MASKEDNUMBER is its closest identifier label, so ID recall on
    # those models is limited by the schema, not only by the model.
    "firstname": "NAME", "lastname": "NAME", "middlename": "NAME",
    "dateofbirth": "DATE",
    "street": "LOCATION", "zipcode": "LOCATION", "buildingnumber": "LOCATION",
    "secondaryaddress": "LOCATION",
    "phone": "PHONE",
    "maskednumber": "ID",
}


def _month_dates(fmt: str, months: list[str]) -> list[str]:
    days = [(12, 3, 1980), (3, 7, 1975), (21, 1, 1992), (8, 10, 1968), (30, 5, 2001),
            (14, 2, 1959), (2, 9, 1987), (27, 11, 1971), (5, 4, 1996), (19, 6, 1983)]
    return [fmt.format(d=d, m=months[m - 1], y=y) for d, m, y in days]


# Per-language templates, names and dates; English uses the module constants.
LANGUAGES = {
    "nl": {
        "names": [("Sanne", "de Vries"), ("Bram", "Jansen"), ("Fleur", "Bakker"), ("Daan", "Visser"),
                  ("Noor", "Smit"), ("Lars", "Mulder"), ("Eva", "Bos"), ("Thijs", "Vos")],
        "dates": _month_dates("{d} {m} {y}", ["januari", "februari", "maart", "april", "mei", "juni", "juli",
                                              "augustus", "september", "oktober", "november", "december"]),
        "cities": ["Utrecht", "Leiden", "Gent", "Zwolle", "Breda"],
        "templates": [
            ["Patiënt ", ("first", "NAME"), " ", ("last", "NAME"), ", geboren op ", ("date", "DATE"),
             ", woont in ", ("city", "LOCATION"), "."],
            [("first", "NAME"), " ", ("last", "NAME"), " kwam op ", ("date", "DATE"),
             " voor controle van diabetes type 2."],
            ["Bel ", ("first", "NAME"), " op ", ("phone", "PHONE"), " of mail naar ", ("email", "EMAIL"), "."],
            ["Dossier ", ("mrn", "ID"), " hoort bij ", ("first", "NAME"), " ", ("last", "NAME"), "."],
            ["De patiënt van ", ("age", "AGE"), " jaar uit ", ("city", "LOCATION"),
             " meldde pijn op de borst op ", ("date", "DATE"), "."],
        ],
    },
    "fr": {
        "names": [("Camille", "Lefèvre"), ("Louis", "Moreau"), ("Inès", "Girard"), ("Hugo", "Lambert"),
                  ("Léa", "Rousseau"), ("Jules", "Fontaine"), ("Manon", "Chevalier"), ("Nathan", "Garnier")],
        "dates": _month_dates("{d} {m} {y}", ["janvier", "février", "mars", "avril", "mai", "juin", "juillet",
                                              "août", "septembre", "octobre", "novembre", "décembre"]),
        "cities": ["Lyon", "Nantes", "Lille", "Rennes", "Dijon"],
        "templates": [
            ["Le patient ", ("first", "NAME"), " ", ("last", "NAME"), ", né le ", ("date", "DATE"),
             ", habite à ", ("city", "LOCATION"), "."],
            [("first", "NAME"), " ", ("last", "NAME"), " a été vu le ", ("date", "DATE"),
             " pour le suivi d'un diabète de type 2."],
            ["Contacter ", ("first", "NAME"), " au ", ("phone", "PHONE"), " ou par courriel à ",
             ("email", "EMAIL"), "."],
            ["Le dossier ", ("mrn", "ID"), " appartient à ", ("first", "NAME"), " ", ("last", "NAME"), "."],
            ["Le patient de ", ("age", "AGE"), " ans venant de ", ("city", "LOCATION"),
             " a signalé une douleur thoracique le ", ("date", "DATE"), "."],
        ],
    },
    "de": {
        "names": [("Lena", "Hoffmann"), ("Jonas", "Schäfer"), ("Mia", "Becker"), ("Felix", "Wagner"),
                  ("Emma", "Schulz"), ("Paul", "Krüger"), ("Hannah", "Zimmermann"), ("Leon", "Braun")],
        "dates": _month_dates("{d}. {m} {y}", ["Januar", "Februar", "März", "April", "Mai", "Juni", "Juli",
                                               "August", "September", "Oktober", "November", "Dezember"]),
        "cities": ["Freiburg", "Kassel", "Leipzig", "Bremen", "Graz"],
        "templates": [
            ["Patient ", ("first", "NAME"), " ", ("last", "NAME"), ", geboren am ", ("date", "DATE"),
             ", wohnt in ", ("city", "LOCATION"), "."],
            [("first", "NAME"), " ", ("last", "NAME"), " kam am ", ("date", "DATE"),
             " zur Kontrolle des Typ-2-Diabetes."],
            [("first", "NAME"), " ist unter ", ("phone", "PHONE"), " oder per E-Mail unter ",
             ("email", "EMAIL"), " erreichbar."],
            ["Die Akte ", ("mrn", "ID"), " gehört zu ", ("first", "NAME"), " ", ("last", "NAME"), "."],
            ["Der ", ("age", "AGE"), " Jahre alte Patient aus ", ("city", "LOCATION"),
             " klagte am ", ("date", "DATE"), " über Brustschmerzen."],
        ],
    },
    "es": {
        "names": [("Lucía", "García"), ("Mateo", "Fernández"), ("Sofía", "López"), ("Hugo", "Martín"),
                  ("Valeria", "Sánchez"), ("Pablo", "Romero"), ("Carmen", "Navarro"), ("Diego", "Torres")],
        "dates": _month_dates("{d} de {m} de {y}", ["enero", "febrero", "marzo", "abril", "mayo", "junio",
                                                    "julio", "agosto", "septiembre", "octubre", "noviembre",
                                                    "diciembre"]),
        "cities": ["Sevilla", "Valencia", "Bilbao", "Granada", "Zaragoza"],
        "templates": [
            ["La paciente ", ("first", "NAME"), " ", ("last", "NAME"), ", nacida el ", ("date", "DATE"),
             ", vive en ", ("city", "LOCATION"), "."],
            [("first", "NAME"), " ", ("last", "NAME"), " acudió el ", ("date", "DATE"),
             " para el control de su diabetes tipo 2."],
            ["Contacte con ", ("first", "NAME"), " en el ", ("phone", "PHONE"), " o en ", ("email", "EMAIL"), "."],
            ["El expediente ", ("mrn", "ID"), " pertenece a ", ("first", "NAME"), " ", ("last", "NAME"), "."],
            ["El paciente de ", ("age", "AGE"), " años de ", ("city", "LOCATION"),
             " refirió dolor torácico el ", ("date", "DATE"), "."],
        ],
    },
    "it": {
        "names": [("Giulia", "Rossi"), ("Lorenzo", "Bianchi"), ("Chiara", "Romano"), ("Matteo", "Ricci"),
                  ("Sara", "Conti"), ("Andrea", "Greco"), ("Elena", "Marino"), ("Luca", "Gallo")],
        "dates": _month_dates("{d} {m} {y}", ["gennaio", "febbraio", "marzo", "aprile", "maggio", "giugno",
                                              "luglio", "agosto", "settembre", "ottobre", "novembre",
                                              "dicembre"]),
        "cities": ["Torino", "Bologna", "Verona", "Bari", "Parma"],
        "templates": [
            ["Il paziente ", ("first", "NAME"), " ", ("last", "NAME"), ", nato il ", ("date", "DATE"),
             ", vive a ", ("city", "LOCATION"), "."],
            [("first", "NAME"), " ", ("last", "NAME"), " è stato visitato il ", ("date", "DATE"),
             " per il controllo del diabete di tipo 2."],
            ["Contattare ", ("first", "NAME"), " al ", ("phone", "PHONE"), " o all'indirizzo ",
             ("email", "EMAIL"), "."],
            ["La cartella ", ("mrn", "ID"), " appartiene a ", ("first", "NAME"), " ", ("last", "NAME"), "."],
            ["Il paziente di ", ("age", "AGE"), " anni di ", ("city", "LOCATION"),
             " ha riferito dolore toracico il ", ("date", "DATE"), "."],
        ],
    },
    "pt": {
        "names": [("Beatriz", "Silva"), ("João", "Santos"), ("Mariana", "Costa"), ("Tiago", "Pereira"),
                  ("Inês", "Oliveira"), ("Rafael", "Martins"), ("Carolina", "Sousa"), ("Miguel", "Rodrigues")],
        "dates": _month_dates("{d} de {m} de {y}", ["janeiro", "fevereiro", "março", "abril", "maio", "junho",
                                                    "julho", "agosto", "setembro", "outubro", "novembro",
                                                    "dezembro"]),
        "cities": ["Porto", "Braga", "Coimbra", "Faro", "Lisboa"],
        "templates": [
            ["A paciente ", ("first", "NAME"), " ", ("last", "NAME"), ", nascida em ", ("date", "DATE"),
             ", mora em ", ("city", "LOCATION"), "."],
            [("first", "NAME"), " ", ("last", "NAME"), " foi consultado em ", ("date", "DATE"),
             " para controlo da diabetes tipo 2."],
            ["Contacte ", ("first", "NAME"), " pelo ", ("phone", "PHONE"), " ou pelo e-mail ",
             ("email", "EMAIL"), "."],
            ["O processo ", ("mrn", "ID"), " pertence a ", ("first", "NAME"), " ", ("last", "NAME"), "."],
            ["O paciente de ", ("age", "AGE"), " anos de ", ("city", "LOCATION"),
             " referiu dor torácica em ", ("date", "DATE"), "."],
        ],
    },
}


def value(field: str, i: int, lang: str = "en") -> str:
    spec = LANGUAGES.get(lang, {})
    names = spec.get("names", NAMES)
    first, last = names[i % len(names)]
    dates = spec.get("dates", DATES)
    cities = spec.get("cities", CITIES)
    return {
        "first": first,
        "last": last,
        "date": dates[i % len(dates)],
        "city": cities[i % len(cities)],
        "phone": PHONES[i % len(PHONES)],
        "email": EMAILS[i % len(EMAILS)],
        "mrn": MRNS[i % len(MRNS)],
        "age": AGES[i % len(AGES)],
    }[field]


def build(lang: str = "en") -> dict:
    templates = LANGUAGES[lang]["templates"] if lang in LANGUAGES else TEMPLATES
    documents = []
    for i in range(40):
        template = templates[i % len(templates)]
        text, spans = "", []
        for part in template:
            if isinstance(part, str):
                text += part
                continue
            field, label = part
            v = value(field, i, lang)
            start = len(text)  # Python str length counts Unicode scalar values
            text += v
            spans.append({"start": start, "end": len(text), "label": label, "text": v})
        documents.append({"id": f"pii-{lang}-{i:02d}", "language": lang, "text": text, "spans": spans})
    return {
        "name": f"medscale-pii-synthetic-{lang}",
        "version": "1",
        "provenance": "Synthetic sentences and identifiers authored for MedScale by "
                      "scripts/evaluation/generate_pii_gold.py; no real person or record.",
        "license": "CC0-1.0",
        "labels": ["AGE", "DATE", "EMAIL", "ID", "LOCATION", "NAME", "PHONE"],
        "label_map": LABEL_MAP,
        "documents": documents,
    }


if __name__ == "__main__":
    out = sys.argv[1]
    lang = sys.argv[2] if len(sys.argv) > 2 else "en"
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        json.dump(build(lang), f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(f"wrote {out}")
