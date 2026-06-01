CREATE TABLE promo_codes (
    code VARCHAR PRIMARY KEY,
    discount_percentage INT NOT NULL,
    duration VARCHAR NOT NULL,
    active BOOLEAN DEFAULT TRUE
);

INSERT INTO promo_codes (code, discount_percentage, duration, active) VALUES
('10-off-pro-1m', 10, '1 month', TRUE),
('15-off-pro-1m', 15, '1 month', TRUE),
('25-off-pro-1m', 25, '1 month', TRUE),
('100-off-pro-1m', 100, '1 month', TRUE);
