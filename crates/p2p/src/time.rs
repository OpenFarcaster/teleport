use crate::errors::*;

const FARCASTER_EPOCH: i64 = 1609459200000;

pub fn get_farcaster_time() -> Result<u32, HubError> {
    to_farcaster_time(
        (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis())
        .try_into()
        .unwrap(),
    )
}

pub fn to_farcaster_time(time: i64) -> Result<u32, HubError> {
    println!("to_farcaster_time: {}", time);
    if time < FARCASTER_EPOCH {
        return Err(HubError::BadRequest(
            BadRequestType::InvalidParam,
            "time must be after Farcaster epoch (01/01/2021".to_string(),
        ));
    }
    let seconds_since_epoch = (time - FARCASTER_EPOCH) / 1000;
    if seconds_since_epoch > 2i64.pow(32) - 1 {
        return Err(HubError::BadRequest(
            BadRequestType::InvalidParam,
            "time too far in future".to_string(),
        ));
    }

    Ok(seconds_since_epoch.try_into().unwrap())
}

pub fn from_farcaster_time(time: u32) -> Result<i64, HubError> {
    Ok(time as i64 * 1000 + FARCASTER_EPOCH)
}
