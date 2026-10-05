import pytest

pytest.importorskip("fastapi")
pytest.importorskip("httpx")

from fastapi.testclient import TestClient  # noqa: E402

from deep_analyzer.app import app  # noqa: E402

client = TestClient(app)


def test_healthz():
    assert client.get("/healthz").status_code == 200


def test_analyze_queues_session():
    res = client.post("/v1/analyze", json={"session_id": "s1", "client_score": 60, "features": [1.0, 2.0]})
    assert res.status_code == 202
    assert res.json() == {"session_id": "s1", "status": "queued"}


def test_rejects_out_of_range_score():
    res = client.post("/v1/analyze", json={"session_id": "s1", "client_score": 101, "features": []})
    assert res.status_code == 422
