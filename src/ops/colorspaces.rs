use crate::opbasics::*;
use crate::color_conversions::*;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct OpToLab {
  pub cam_to_xyz: [[f32;4];3],
  pub cam_to_xyz_normalized: [[f32;4];3],
  pub xyz_to_cam: [[f32;3];4],
  pub wb_coeffs: [f32;4],
}

fn normalize_wbs(vals: [f32;4]) -> [f32;4] {
  // Set green multiplier as 1.0
  let unity: f32 = vals[1];

  macro_rules! norm {
    ($val:expr) => {
      if !$val.is_normal() {
        1.0
      } else {
        $val / unity
      }
    };
  }

  [norm!(vals[0]), norm!(vals[1]), norm!(vals[2]), norm!(vals[3])]
}

impl OpToLab {
  pub fn new(img: &ImageSource) -> OpToLab {
    match img {
      ImageSource::Raw(img) => {
        let coeffs = if !img.wb_coeffs[0].is_normal() ||
                        !img.wb_coeffs[1].is_normal() ||
                        !img.wb_coeffs[2].is_normal() {
          normalize_wbs(img.neutralwb())
        } else {
          normalize_wbs(img.wb_coeffs)
        };
/*        println!("Camera: {}", img.camera.model.to_string());
        println!("camera xyz: {:?}", img.camera.xyz_to_cam);
        println!("cam_to_xyz: {:?}", img.cam_to_xyz());
        println!("cam_to_xyz_normalized: {:?}", img.cam_to_xyz_normalized());
        println!("SRGB: {:?}", *SRGB_D65_43);
        println!("xyz_to_cam: {:?}", img.xyz_to_cam);
        println!("wb_coeffs: {:?}", coeffs);
        println!("Rotation: {:?}", img.orientation);
*/
        OpToLab{
          cam_to_xyz: img.cam_to_xyz(), //*SRGB_D65_43,
          cam_to_xyz_normalized: img.cam_to_xyz_normalized(), //*SRGB_D65_43,
          xyz_to_cam: img.xyz_to_cam,
          wb_coeffs: coeffs,
        }
      },
      ImageSource::Other(_) => {
        OpToLab{
          cam_to_xyz: *SRGB_D65_43,
          cam_to_xyz_normalized: *SRGB_D65_43,
          xyz_to_cam: *XYZ_D65_34,
          wb_coeffs: [1.0, 1.0, 1.0, 0.0],
        }
      }
    }
  }

  pub fn set_temp(&mut self, temp: f32, tint: f32) {
    let xyz = temp_to_xyz(temp);
    let xyz = [xyz[0], xyz[1]/tint, xyz[2]];
    for i in 0..4 {
      self.wb_coeffs[i] = 0.0;
      for j in 0..3 {
        self.wb_coeffs[i] += self.xyz_to_cam[i][j] * xyz[j];
      }
      self.wb_coeffs[i] = self.wb_coeffs[i].recip();
    }
    self.wb_coeffs = normalize_wbs(self.wb_coeffs);
  }

  pub fn get_temp(&self) -> (f32, f32) {
    let mut xyz = [0.0; 3];
    for i in 0..3 {
      for j in 0..4 {
        let mul = self.wb_coeffs[j];
        if mul > 0.0 {
          xyz[i] += self.cam_to_xyz[i][j] / mul;
        }
      }
    }
    let (temp, tint) = xyz_to_temp(xyz);
    (temp, tint)
  }
}

impl<'a> ImageOp<'a> for OpToLab {
  fn name(&self) -> &str {"to_lab"}
  fn run(&self, _pipeline: &PipelineGlobals, buf: Arc<OpBuffer>) -> Arc<OpBuffer> {
    let cmatrix = if buf.monochrome {
      // Monochrome means we don't need color conversion so it's as if the camera is itself D65 SRGB
      *SRGB_D65_43
    } else {
      self.cam_to_xyz_normalized
    };

    let mul = if buf.monochrome {
      [1.0, 1.0, 1.0, 1.0]
    } else {
      normalize_wbs(self.wb_coeffs)
    };

    Arc::new(buf.process_into_new(3, &(|outb: &mut [f32], inb: &[f32]| {
      for (pixin, pixout) in inb.chunks_exact(4).zip(outb.chunks_exact_mut(3)) {
        let (l,a,b) = camera_to_lab(mul, cmatrix, pixin);
        pixout[0] = l;
        pixout[1] = a;
        pixout[2] = b;
      }
    })))
  }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct OpFromLab {
}

impl OpFromLab {
  pub fn new(_img: &ImageSource) -> OpFromLab {
    OpFromLab{}
  }
}

impl<'a> ImageOp<'a> for OpFromLab {
  fn name(&self) -> &str {"from_lab"}
  fn run(&self, _pipeline: &PipelineGlobals, buf: Arc<OpBuffer>) -> Arc<OpBuffer> {
    Arc::new(buf.mutate_lines_copying(&(|line: &mut [f32], _| {
      for pix in line.chunks_exact_mut(3) {
        let (r,g,b) = lab_to_rgb(*XYZ_D65_33, pix);

        pix[0] = r;
        pix[1] = g;
        pix[2] = b;
      }
    })))
  }
}
