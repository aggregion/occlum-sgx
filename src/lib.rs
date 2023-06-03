use std::fmt::Debug;
use std::mem::size_of;
use std::ops::Deref;

#[macro_use]
extern crate lazy_static;

use error::SGXError;
use ioctl::IOCTL_CLIENT;
use log::warn;
pub use types::*;

mod constants;
mod error;
mod ioctl;
mod types;

pub struct SGXQuote {
    buf: Vec<u8>,
    report_body: *const SGXReportBody,
}

impl TryFrom<Vec<u8>> for SGXQuote {
    type Error = SGXError;
    fn try_from(buf: Vec<u8>) -> Result<Self, Self::Error> {
        let report_body_offset = size_of::<SGXQuoteHeader>();
        let report_body_size = size_of::<SGXReportBody>();

        if buf.len() < report_body_offset + report_body_size {
            return Err(SGXError::BadQuoteLength {
                min: buf.len(),
                actual: report_body_offset + report_body_size,
            });
        }

        let report_body = buf.as_slice()[report_body_offset..].as_ptr() as *const SGXReportBody;

        Ok(Self { buf, report_body })
    }
}

impl TryFrom<&[u8]> for SGXQuote {
    type Error = SGXError;

    fn try_from(buf: &[u8]) -> Result<Self, Self::Error> {
        buf.to_vec().try_into()
    }
}

impl TryFrom<ReportData> for SGXQuote {
    type Error = SGXError;

    fn try_from(value: ReportData) -> Result<Self, Self::Error> {
        Self::generate(&value)
    }
}

impl Deref for SGXQuote {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.buf.as_ref()
    }
}

impl SGXQuote {
    pub fn generate(data: &ReportData) -> Result<Self, SGXError> {
        let result = IOCTL_CLIENT
            .lock()
            .unwrap()
            .generate_quote(SGXReportData::new(data.clone()))?;
        Ok(result.try_into()?)
    }

    pub fn as_slice(&self) -> &[u8] {
        &*self
    }

    pub fn verify(&self) -> Result<bool, SGXError> {
        let result = IOCTL_CLIENT
            .lock()
            .unwrap()
            .verify_quote(self.buf.as_ref())?;

        match result {
            SGXQuoteVerifyResult::Ok => Ok(true),
            SGXQuoteVerifyResult::ConfigNeeded
            | SGXQuoteVerifyResult::OutOfDate
            | SGXQuoteVerifyResult::OutOfDateConfigNeeded
            | SGXQuoteVerifyResult::SwHardeningNeeded
            | SGXQuoteVerifyResult::ConfigAndSwHardeningNeeded => {
                warn!(
                    "SGX Quote Verification completed with non-terminal result: {:?}",
                    result
                );
                Ok(true)
            }
            _ => Err(SGXError::VerifyQuoteFailed(result)),
        }
    }

    pub fn isv_family_id(&self) -> SGXFamilyId {
        unsafe { (*self.report_body).isv_family_id }
    }

    pub fn isv_ext_prod_id(&self) -> SGXExtProdId {
        unsafe { (*self.report_body).isv_ext_prod_id }
    }

    pub fn config_id(&self) -> SGXConfigId {
        unsafe { (*self.report_body).config_id }
    }

    pub fn mrenclave(&self) -> SGXMeasurement {
        unsafe { (*self.report_body).mr_enclave }
    }

    pub fn mrsigner(&self) -> SGXMeasurement {
        unsafe { (*self.report_body).mr_signer }
    }

    pub fn product_id(&self) -> u16 {
        unsafe { (*self.report_body).isv_prod_id }
    }

    pub fn version(&self) -> u16 {
        unsafe { (*self.report_body).isv_svn }
    }

    pub fn report_data(&self) -> SGXReportData {
        unsafe { (*self.report_body).report_data }
    }
}

impl Debug for SGXQuote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SGXQuote")
            .field("mrenclave", &self.mrenclave())
            .field("mrsigner", &self.mrsigner())
            .field("report_body", &self.report_data())
            .field("product_id", &self.product_id())
            .field("version", &self.version())
            .field("family_id", &self.isv_family_id())
            .field("ext_prod_id", &self.isv_ext_prod_id())
            .field("config_id", &self.config_id())
            .finish()
    }
}
