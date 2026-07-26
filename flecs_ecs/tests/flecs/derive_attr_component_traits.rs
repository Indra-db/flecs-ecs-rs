#![allow(dead_code)]
use crate::common_test::*;
use flecs_ecs::experimental::prelude::EntityGuardExt;

mod component_traits_attributes {
    use super::*;

    #[derive(Component)]
    struct _Y;

    #[derive(Component)]
    struct Group;

    // Single-trait components
    #[derive(Component)]
    #[flecs(traits(Transitive))]
    struct TTransitive;

    #[derive(Component)]
    #[flecs(traits(Reflexive))]
    struct TReflexive;

    #[derive(Component)]
    #[flecs(traits(Symmetric))]
    struct TSymmetric;

    #[derive(Component)]
    #[flecs(traits(Final))]
    struct TFinal;

    #[derive(Component)]
    #[flecs(traits(Inheritable))]
    struct TInheritable;

    #[derive(Component)]
    #[flecs(traits(PairIsTag))]
    struct TPairIsTag;

    #[derive(Component)]
    #[flecs(traits(Exclusive))]
    struct TExclusive;

    #[derive(Component)]
    #[flecs(traits(Acyclic))]
    struct TAcyclic;

    #[derive(Component)]
    #[flecs(traits(Traversable))]
    struct TTraversable;

    #[derive(Component)]
    #[flecs(traits(CanToggle))]
    struct TCanToggle;

    #[derive(Component)]
    #[flecs(traits(Trait))]
    struct TTrait;

    #[derive(Component)]
    #[flecs(traits(Relationship))]
    struct TRelationship;

    #[derive(Component)]
    #[flecs(traits(Target))]
    struct TTarget;

    #[derive(Component)]
    #[flecs(traits(Sparse))]
    struct TSparse;

    #[derive(Component)]
    #[flecs(traits(DontFragment))]
    struct TDontFragment;

    // Pair-trait components (relationship, target)
    #[derive(Component)]
    #[flecs(traits((With, _Y)))]
    struct TWithY;

    #[derive(Component)]
    #[flecs(traits((OneOf, Group)))]
    struct TOneOfGroup;

    #[derive(Component)]
    #[flecs(traits((OnInstantiate, Override)))]
    struct TOnInstOverride;

    #[derive(Component)]
    #[flecs(traits((OnInstantiate, Inherit)))]
    struct TOnInstInherit;

    #[derive(Component)]
    #[flecs(traits((OnInstantiate, DontInherit)))]
    struct TOnInstDontInherit;

    #[derive(Component)]
    #[flecs(traits(Acyclic,(OnInstantiate,Inherit),Inheritable,flecs::Sparse))]
    struct MultipleTraits;

    #[test]
    fn component_traits_flags_present() {
        let world = World::new();

        // Singles
        let c = world.component::<TTransitive>();
        assert!(c.has(flecs::Transitive));
        let c = world.component::<TReflexive>();
        assert!(c.has(flecs::Reflexive));
        let c = world.component::<TSymmetric>();
        assert!(c.has(flecs::Symmetric));
        let c = world.component::<TFinal>();
        assert!(c.has(flecs::Final));
        let c = world.component::<TInheritable>();
        assert!(c.has(flecs::Inheritable));
        let c = world.component::<TPairIsTag>();
        assert!(c.has(flecs::PairIsTag));
        let c = world.component::<TExclusive>();
        assert!(c.has(flecs::Exclusive));
        let c = world.component::<TAcyclic>();
        assert!(c.has(flecs::Acyclic));
        let c = world.component::<TTraversable>();
        assert!(c.has(flecs::Traversable));
        let c = world.component::<TCanToggle>();
        assert!(c.has(flecs::CanToggle));
        let c = world.component::<TTrait>();
        assert!(c.has(flecs::Trait));
        let c = world.component::<TRelationship>();
        assert!(c.has(flecs::Relationship));
        let c = world.component::<TTarget>();
        assert!(c.has(flecs::Target));
        let c = world.component::<TSparse>();
        assert!(c.has(flecs::Sparse));
        let c = world.component::<TDontFragment>();
        assert!(c.has(flecs::DontFragment));

        // Pairs
        let c = world.component::<TWithY>();
        assert!(c.has((flecs::With, _Y)));

        let c = world.component::<TOneOfGroup>();
        assert!(c.has((flecs::OneOf, self::Group)));

        let c = world.component::<TOnInstOverride>();
        assert!(c.has((flecs::OnInstantiate, flecs::Override)));
        let c = world.component::<TOnInstInherit>();
        assert!(c.has((flecs::OnInstantiate, flecs::Inherit)));
        let c = world.component::<TOnInstDontInherit>();
        assert!(c.has((flecs::OnInstantiate, flecs::DontInherit)));

        // Multiple traits
        let c = world.component::<MultipleTraits>();
        assert!(c.has(flecs::Acyclic));
        assert!(c.has((flecs::OnInstantiate, flecs::Inherit)));
        assert!(c.has(flecs::Inheritable));
        assert!(c.has(flecs::Sparse));
    }
}
mod name_attribute {
    use super::*;

    #[derive(Component)]
    #[flecs(meta, name = "AName")]
    struct CompileTestOrdering;

    #[derive(Component)]
    #[flecs(name = "AName", meta)]
    struct CompileTestOrdering2;

    #[derive(Component)]
    #[flecs(name = "AName", meta, traits(DontFragment, flecs::Sparse))]
    struct CompileTestMultipleFlecsAttributes;

    #[derive(Component)]
    #[flecs(name = "AName")]
    struct CompileTestNameAttribute;

    #[test]
    fn der_attr_name_setting() {
        let world = World::new();

        let c = world.component::<CompileTestNameAttribute>();

        assert_eq!(c.name(), "AName");
    }
}

mod add_set_attributes {
    use super::*;

    #[derive(Component)]
    struct TAdd;

    #[derive(Component, Default)]
    struct CAdd;

    #[derive(Component, Default)]
    struct CSetWDefault {
        value: u32,
    }

    #[derive(Component, Default)]
    struct CSet1F {
        value: u32,
    }

    #[derive(Component, Default)]
    struct CSet2F {
        value: u32,
        other: u32,
    }

    #[derive(Component)]
    struct IsATarget(i32);

    #[derive(Component)]
    struct ChildOfTarget;

    #[derive(Component)]
    #[flecs(add((flecs::IsA, IsATarget)))]
    struct TIsA;

    #[derive(Component)]
    #[flecs(add((flecs::ChildOf, ChildOfTarget)))]
    struct TChildOf;

    #[test]
    fn child_of_isa_traits_flags_present() {
        let world = World::new();

        let c = world.component::<TIsA>();
        assert!(c.has((flecs::IsA, IsATarget::id())));
        let c = world.component::<TChildOf>();
        assert!(c.has((flecs::ChildOf, ChildOfTarget)));
    }

    #[derive(Component)]
    #[flecs(
        traits(Prefab), //flecs trait to say it's a prefab
        add(TAdd, CAdd,
            (TAdd, CAdd), //pair
            (flecs::IsA, IsATarget)
            ),
        set(CSet1F { value: 1 }, CSet2F { value: 2, other: 3 }, // inline construction
            CSetWDefault::default(),  //constructed from fn
            (CSet1F { value: 4 }, CSet2F), //pair
            (TAdd, CSet1F { value: 5 }) //pair
        )
    )]
    struct TestAddSet;

    #[test]
    fn add_set_attr() {
        let world = World::new();

        let c = world.component::<TestAddSet>();
        assert!(c.has(flecs::Prefab));
        assert!(c.has(TAdd::id()));
        assert!(c.has(CAdd::id()));
        assert!(c.has((TAdd::id(), CAdd::id())));

        {
            let (set1, set2, setdef, setpair, setpair2) = c
                .get_ref::<(
                    &CSet1F,
                    &CSet2F,
                    &CSetWDefault,
                    &(CSet1F, CSet2F),
                    &(TAdd, CSet1F),
                )>()
                .unwrap();
            assert_eq!(set1.value, 1);
            assert_eq!(set2.value, 2);
            assert_eq!(set2.other, 3);
            assert_eq!(setdef.value, 0);
            assert_eq!(setpair.value, 4);
            assert_eq!(setpair2.value, 5);
        };

        //internally it does
        // world
        //     .component::<TestAddSet>()
        //     .add((TAdd::id(), CAdd::id()));
        //     .add(TAdd::id())
        //     .add(CAdd::id())
        //     .set(CSet1F { value: 1 })
        //     .set(CSet2F { value: 2, other: 3 })
        //     .set_first(CSet1F { value: 4 }, CSet2F)
        //     .set_second(CTAdd, CSet1F { value: 5 });
    }
}

mod component_hooks_attributes {

    use super::*;

    #[derive(Default, Component)]
    #[flecs(hooks(on_add(on_add_hook)))]
    struct OnAddHookFn(i32);

    #[derive(Default, Component)]
    #[flecs(hooks(on_add(|e, _c| {
        {
            let w = e.world();
            let mut counter = w
                .entity_from_id(OnAddHookCounter::entity_id(w))
                .get_ref::<&mut OnAddHookCounter>()
                .unwrap();
            counter.count += 1;
        }
    })))]
    struct OnAddHookInline(i32);

    #[derive(Default, Component)]
    #[flecs(hooks(on_remove(on_remove_hook)))]
    struct OnRemoveHookFn(i32);

    #[derive(Default, Component)]
    #[flecs(hooks(on_remove(|e, _c| {
        {
            let w = e.world();
            let mut counter = w
                .entity_from_id(OnRemoveHookCounter::entity_id(w))
                .get_ref::<&mut OnRemoveHookCounter>()
                .unwrap();
            counter.count += 1;
        }
    })))]
    struct OnRemoveHookInline(i32);

    #[derive(Default, Component)]
    #[flecs(hooks(on_set(on_set_hook)))]
    struct OnSetHookFn(i32);

    #[derive(Default, Component)]
    #[flecs(hooks(on_set(|e, _c| {
        {
            let w = e.world();
            let mut counter = w
                .entity_from_id(OnSetHookCounter::entity_id(w))
                .get_ref::<&mut OnSetHookCounter>()
                .unwrap();
            counter.count += 1;
        }
    })))]
    struct OnSetHookInline(i32);

    #[derive(Default, Component)]
    #[flecs(hooks(on_replace(on_replace_hook)))]
    struct OnReplaceHookFn(i32);

    #[derive(Default, Component)]
    #[flecs(hooks(on_replace(|e, _p,_n| {
        {
            let w = e.world();
            let mut counter = w
                .entity_from_id(OnReplaceHookCounter::entity_id(w))
                .get_ref::<&mut OnReplaceHookCounter>()
                .unwrap();
            counter.count += 1;
        }
    })))]
    struct OnReplaceHookInline(i32);

    fn on_add_hook(e: EntityView<'_>, _c: &mut OnAddHookFn) {
        {
            let w = e.world();
            let mut counter = w
                .entity_from_id(OnAddHookCounter::entity_id(w))
                .get_ref::<&mut OnAddHookCounter>()
                .unwrap();
            counter.count += 1;
        }
    }

    fn on_set_hook(e: EntityView<'_>, _c: &mut OnSetHookFn) {
        {
            let w = e.world();
            let mut counter = w
                .entity_from_id(OnSetHookCounter::entity_id(w))
                .get_ref::<&mut OnSetHookCounter>()
                .unwrap();
            counter.count += 1;
        }
    }

    fn on_remove_hook(e: EntityView<'_>, _c: &mut OnRemoveHookFn) {
        {
            let w = e.world();
            let mut counter = w
                .entity_from_id(OnRemoveHookCounter::entity_id(w))
                .get_ref::<&mut OnRemoveHookCounter>()
                .unwrap();
            counter.count += 1;
        }
    }

    fn on_replace_hook(e: EntityView<'_>, _p: &mut OnReplaceHookFn, _n: &mut OnReplaceHookFn) {
        {
            let w = e.world();
            let mut counter = w
                .entity_from_id(OnReplaceHookCounter::entity_id(w))
                .get_ref::<&mut OnReplaceHookCounter>()
                .unwrap();
            counter.count += 1;
        }
    }

    #[derive(Component, Clone, Default)]
    struct OnAddHookCounter {
        count: u32,
    }

    #[derive(Component, Clone, Default)]
    struct OnSetHookCounter {
        count: u32,
    }

    #[derive(Component, Clone, Default)]
    struct OnRemoveHookCounter {
        count: u32,
    }

    #[derive(Component, Clone, Default)]
    struct OnReplaceHookCounter {
        count: u32,
    }

    #[test]
    fn component_hooks_attr() {
        let world = World::new();

        world.add(OnAddHookCounter::id());
        world.add(OnSetHookCounter::id());
        world.add(OnRemoveHookCounter::id());
        world.add(OnReplaceHookCounter::id());

        let e = world
            .entity()
            .add(OnAddHookFn::id())
            .add(OnAddHookInline::id())
            .add(OnRemoveHookFn::id())
            .add(OnRemoveHookInline::id())
            .set(OnSetHookFn::default())
            .set(OnSetHookInline::default())
            .add(OnReplaceHookFn::id())
            .add(OnReplaceHookInline::id())
            .remove(OnRemoveHookFn::id())
            .remove(OnRemoveHookInline::id());

        let c_add = world.entity_from_id(OnAddHookCounter::entity_id(&world)).cloned_owned::<&OnAddHookCounter>().unwrap();
        assert_eq!(c_add.count, 2, "Expected 2 OnAddHook calls");
        let c_set = world.entity_from_id(OnSetHookCounter::entity_id(&world)).cloned_owned::<&OnSetHookCounter>().unwrap();
        assert_eq!(c_set.count, 2, "Expected 2 OnSetHook calls");
        let c_remove = world.entity_from_id(OnRemoveHookCounter::entity_id(&world)).cloned_owned::<&OnRemoveHookCounter>().unwrap();
        assert_eq!(c_remove.count, 2, "Expected 2 OnRemoveHook calls");

        e.set(OnReplaceHookFn::default())
            .set(OnReplaceHookInline::default());

        let c_replace = world.entity_from_id(OnReplaceHookCounter::entity_id(&world)).cloned_owned::<&OnReplaceHookCounter>().unwrap();
        assert_eq!(c_replace.count, 2, "Expected 2 OnReplaceHook calls");
    }
}

mod multi_item_and_trailing_comma_attributes {
    use super::*;

    #[derive(Component)]
    struct TTrail;

    #[derive(Component, Default)]
    struct CTrail {
        value: u32,
    }

    #[derive(Component)]
    #[flecs(
        traits(Transitive, flecs::Sparse,),
        add(TTrail,),
        set(CTrail { value: 7 },)
    )]
    struct TrailingCommas;

    #[test]
    fn der_attr_multi_item_and_trailing_commas() {
        let world = World::new();

        let c = world.component::<TrailingCommas>();
        assert!(c.has(flecs::Transitive));
        assert!(c.has(flecs::Sparse));
        assert!(c.has(TTrail::id()));
        {
            let v = c.get_ref::<&CTrail>().unwrap();
            assert_eq!(v.value, 7);
        };
    }
}
