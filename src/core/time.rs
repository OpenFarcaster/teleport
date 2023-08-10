use super::errors::*;

const FARCASTER_EPOCH: u64 = 1609459200000; // January 1, 2021 UTC

pub fn get_farcaster_time() -> Result<u64, HubError> {
    to_farcaster_time(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    )
}

pub fn to_farcaster_time(time: u64) -> Result<u64, HubError> {
    if time < FARCASTER_EPOCH {
        return Err(HubError::BadRequest(
            BadRequestType::InvalidParam,
            "time must be after Farcaster epoch (01/01/2022)".to_string(),
        ));
    }
    let seconds_since_epoch = (time - FARCASTER_EPOCH) / 1000;
    if seconds_since_epoch > 2u64.pow(32) - 1 {
        return Err(HubError::BadRequest(
            BadRequestType::InvalidParam,
            "time too far in future".to_string(),
        ));
    }

    Ok(seconds_since_epoch)
}

pub fn from_farcaster_time(time: u64) -> Result<u64, HubError> {
    Ok(time * 1000 + FARCASTER_EPOCH)
}
