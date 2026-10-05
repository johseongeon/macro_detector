"""Stage 3 정밀 분석 서버.

verify-api가 '관찰' 단계 세션을 비동기로 넘긴다. 예매 오픈 전 대기열 단계에서
분석을 끝내 실제 예매 순간에는 지연이 생기지 않게 하는 것이 목표다.

실행: uvicorn deep_analyzer.app:app --port 8081
"""

from enum import Enum

from fastapi import FastAPI
from pydantic import BaseModel, Field

app = FastAPI(title="Guard Deep Analyzer", version="0.1.0")


class AnalyzeRequest(BaseModel):
    session_id: str
    client_score: int = Field(ge=0, le=100)
    features: list[float]


class AnalysisStatus(str, Enum):
    queued = "queued"


class AnalyzeResponse(BaseModel):
    session_id: str
    status: AnalysisStatus


@app.get("/healthz")
def healthz() -> str:
    return "ok"


@app.post("/v1/analyze", response_model=AnalyzeResponse, status_code=202)
def analyze(req: AnalyzeRequest) -> AnalyzeResponse:
    # TODO(Phase 4): 작업 큐에 등록 → 원시 궤적 기반 1D-CNN/GRU 추론,
    #                같은 디바이스·IP 대역의 교차 세션 분석, 결과를 라벨 저장소에 기록.
    return AnalyzeResponse(session_id=req.session_id, status=AnalysisStatus.queued)
