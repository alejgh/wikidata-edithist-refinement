from dataclasses import dataclass
from typing import List

@dataclass
class Triple:
    subj: str
    pred: str
    obj: str

@dataclass
class RevisionItem:
    timestamp: str
    flags: List[str]
    tags: List[str]
    user: str
    comment: str
    additions: List[Triple]
    deletions: List[Triple]

@dataclass
class KGEntity:
    name: str
    uri: str
    pagerank_score: float
    revision_history: List[RevisionItem]

@dataclass
class KGClass:
    name: str
    uri: str
    num_instances: int
    classrank_score: float
    top_entities: List[KGEntity]
