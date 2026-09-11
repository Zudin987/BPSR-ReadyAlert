
#[cfg(test)]
mod v1190_release_regression_tests {
    use super::*;
    #[test]
    fn dps_floor_does_not_change_mechanics_or_coordinate_math() {
        assert_eq!(clamp_kind_scale(Kind::Dps,30),50);
        assert_eq!(clamp_kind_scale(Kind::Mechanics,30),30);
        assert_eq!(physical_extent_to_logical(195,30),650);
        assert_eq!((overlay_min_width(Kind::Mechanics,30),overlay_min_height(Kind::Mechanics,30)),(102,78));
        assert_eq!(dps_readability_percent(30),75);
    }
    #[test]
    fn persisted_legacy_dps_bounds_migrate_without_changing_mechanics() {
        let mut file=OverlayScaleFile::default();
        file.dps=OverlayScaleSlot{percent:30,x:10,y:20,width:195,height:120,has_bounds:true};
        file.mechanics=file.dps.clone();normalize_scale_file(&mut file);
        assert_eq!((file.dps.percent,file.dps.width,file.dps.height),(50,325,200));
        assert_eq!((file.mechanics.percent,file.mechanics.width,file.mechanics.height),(30,195,120));
        normalize_scale_file(&mut file);assert_eq!(file.dps.width,325);
    }
    #[test]
    fn measured_spec_has_priority_over_ability_and_long_username() { unsafe {
        let dc=CreateCompatibleDC(null_mut());assert!(!dc.is_null());
        let font=GetStockObject(DEFAULT_GUI_FONT);let old=SelectObject(dc,font);
        let row=DpsRow{name:"VeryLongPlayerNameRepeatedRepeatedRepeated".into(),subprofession_name:"Smite".into(),ability_score:58_000,illusion_break:3210,..DpsRow::default()};
        let spec_width=dps_text_width(dc,"Smite",100);
        assert_eq!(dps_secondary_compact_measured(dc,&row,spec_width),"Smite");
        assert_eq!(dps_secondary_compact_measured(dc,&row,1),"Smite");
        let rect=RECT{left:6,top:0,right:506,bottom:30};
        let layout=dps_row_layout_responsive(dc,rect,100,true,true,true,true,&row,font,font);
        assert!(layout.spec_right-layout.spec_left>=spec_width);
        assert!(layout.name_right<layout.spec_left);
        let released=dps_row_layout_responsive(dc,rect,100,true,false,false,false,&row,font,font);
        assert_eq!(released.active_left,released.active_right);
        assert_eq!(released.share_left,released.share_right);
        assert_eq!(released.death_left,released.death_right);
        assert!(released.spec_right>layout.spec_right);
        SelectObject(dc,old);DeleteDC(dc);
    }}
    #[test]
    fn scaled_rows_badges_and_page_capacity_agree() {
        for scale in [50,60,80,99,100,150,300] {
            let row=dps_row_h(scale);
            assert!(dps_badge_h(scale)<=row-2);
            assert!(dps_consumable_icon(scale)<=row-2);
            assert_eq!(visible_dps_rows_scaled(dps_rows_top()+row*4,scale),4);
            let actions=toolbar_action_rects_responsive(500,scale);
            for pair in actions.windows(2){assert!(pair[0].0.right<=pair[1].0.left);}
            assert!(actions[0].0.left>=0);
        }
    }
}
