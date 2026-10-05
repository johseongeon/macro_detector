# Testbed — 모의 예매 사이트

데이터 수집과 매크로 재현에 쓰는 정적 페이지입니다.

```bash
python3 -m http.server 5173 --directory testbed
```

보안 브라우저에서 `http://localhost:5173/`을 열면 수집기가 동작합니다(허용 호스트: localhost).
