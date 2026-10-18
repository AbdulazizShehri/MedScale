"""Generate MedScale's synthetic PII gold set (deterministic, no randomness).

Every name, date, place, number and address below is invented for MedScale
and refers to no real person. Offsets are Unicode scalar values, computed
from the assembled text, so the gold spans are exact by construction.

Usage: generate_pii_gold.py OUT_JSON
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
}


def value(field: str, i: int) -> str:
    first, last = NAMES[i % len(NAMES)]
    return {
        "first": first,
        "last": last,
        "date": DATES[i % len(DATES)],
        "city": CITIES[i % len(CITIES)],
        "phone": PHONES[i % len(PHONES)],
        "email": EMAILS[i % len(EMAILS)],
        "mrn": MRNS[i % len(MRNS)],
        "age": AGES[i % len(AGES)],
    }[field]


def build() -> dict:
    documents = []
    for i in range(40):
        template = TEMPLATES[i % len(TEMPLATES)]
        text, spans = "", []
        for part in template:
            if isinstance(part, str):
                text += part
                continue
            field, label = part
            v = value(field, i)
            start = len(text)  # Python str length counts Unicode scalar values
            text += v
            spans.append({"start": start, "end": len(text), "label": label, "text": v})
        documents.append({"id": f"pii-en-{i:02d}", "language": "en", "text": text, "spans": spans})
    return {
        "name": "medscale-pii-synthetic-en",
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
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        json.dump(build(), f, ensure_ascii=False, indent=1)
        f.write("\n")
    print(f"wrote {out}")
